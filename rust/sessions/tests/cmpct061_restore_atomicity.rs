//! Feature: spec/features/session-checkpoint-restore-atomicity.feature

#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "cmpct060/support.rs"]
mod support;

use codelet_core::persistence::compaction_commit::{commit_compaction, publish_compaction};
use codelet_core::persistence::*;
use codelet_core::SessionManagerHandle;
use codelet_providers::codex::session::CodexSession;
use codelet_providers::session::ProviderSessionExtension;
use serde_json::{json, Value};
use std::sync::Arc;

async fn exercise(failure: &str) -> support::TestResult {
    if let Some(session_id) = support::cold_process_session()? {
        return probe(session_id, failure).await;
    }
    // @step Given a fresh-process session with a committed generation and distinct existing runtime state
    let (directory, manager, session_id, _) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "covered original")?;
    let checkpoint = Arc::new(CodexSession::default());
    let mut inner = background.inner.lock().await;
    inner.provider_session.register("codex", checkpoint);
    let model = inner.current_model_id().ok_or("selected model")?;
    let mut candidate = boundary::candidate(&manifest)?;
    candidate.source = inner.capture_durable_compaction(manifest.id)?;
    candidate.checkpoint.model = model.clone();
    candidate.checkpoint.payload =
        serde_json::to_value(inner.provider_session.prepare_generation(1, &model)?)?;
    drop(inner);
    let promoted = commit_compaction(candidate, &model, |_, _| Ok(()))?;
    publish_compaction(&promoted, |_| Ok(()))?;
    manifest = load_session(manifest.id)?;
    set_session_tokens(
        &mut manifest,
        &TokenUsage {
            current_context_tokens: 333,
            cumulative_billed_input: 5030,
            cumulative_billed_output: 2900,
            cache_read_tokens: 43,
            cache_creation_tokens: 17,
            reasoning_tokens: 1100,
        },
    )?;
    append_message_with_metadata(
        &mut manifest,
        "assistant",
        "",
        [(
            "codexContinuation".into(),
            retained_checkpoint(failure, &model)?,
        )]
        .into(),
    )?;
    drop(background);
    drop(manager);
    let scenario = match failure {
        "unsupported version" => "unsupported_version_preserves_session_state",
        "unsupported protocol" => "unsupported_protocol_preserves_session_state",
        "null checkpoint" => "null_checkpoint_preserves_session_state",
        "future generation" => "future_generation_preserves_session_state",
        _ => return Err("unknown checkpoint failure".into()),
    };
    support::run_cold_process(directory.path(), &session_id, scenario).await
}

async fn probe(session_id: codelet_rpc_types::SessionId, failure: &str) -> support::TestResult {
    let manifest = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    let model = manifest
        .compaction_commit
        .as_ref()
        .ok_or("committed boundary")?
        .checkpoint
        .model
        .clone();
    let manager = codelet_sessions::SessionManager::new();
    manager
        .create_session_from_manifest(&manifest, "anthropic/claude-sonnet-4")
        .await?;
    let background = manager.get_session(&session_id.value)?;
    let checkpoint = Arc::new(CodexSession::default());
    let original = json!({"version":1,"generation":0,"protocol":"full-current-dag",
        "model":model,"items":[{"type":"message","role":"user","content":"runtime checkpoint"}]});
    checkpoint.restore(&[json!({"codexContinuation":original}).to_string()])?;
    {
        let mut inner = background.inner.lock().await;
        inner.provider_session.register("codex", checkpoint.clone());
        inner
            .messages
            .push(rig::message::Message::user("runtime prefix"));
        inner.token_tracker.input_tokens = 71;
        inner.token_tracker.output_tokens = 17;
        inner.token_tracker.reasoning_tokens = 9;
        assert!(inner.ensure_compaction_inference_ready().is_ok());
    }
    background.handle_output(codelet_rpc_types::StreamChunk::text("prior display".into()));
    let invalid = get_session_message_envelopes(manifest.id)?;
    let before = state(&background, &checkpoint).await?;
    let mut events = background.subscribe_to_stream();
    // @step When persisted retained checkpoint metadata has <failure>
    let result = manager.restore_session_messages(&session_id, invalid);
    assert!(
        result.is_err(),
        "invalid retained checkpoint must fail: {failure}"
    );
    // @step Then restore fails and session inference remains blocked
    assert!(
        background
            .inner
            .lock()
            .await
            .ensure_compaction_inference_ready()
            .is_err(),
        "failed restore reopened session inference: {failure}"
    );
    // @step And messages checkpoint counters replay installation and output remain unchanged
    assert_eq!(
        state(&background, &checkpoint).await?,
        before,
        "partial restore: {failure}"
    );
    // @step And no physical request or completion is produced
    assert!(
        events.try_recv().is_err(),
        "failed restore published output"
    );
    assert_eq!(load_session(manifest.id)?.revision, manifest.revision);
    // @step When the authoritative valid replay is restored
    update_message_metadata(
        manifest
            .messages
            .last()
            .ok_or("retained checkpoint reference")?
            .message_id,
        [(
            "codexContinuation".into(),
            json!({"version":1,"generation":1,
            "protocol":"full-current-dag","model":model,"items":[]}),
        )]
        .into(),
    )?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(manifest.id)?)?;
    // @step Then the saved generation counters and history become usable
    let inner = background.inner.lock().await;
    assert!(inner.ensure_compaction_inference_ready().is_ok());
    assert_eq!(inner.token_tracker.input_tokens, 333);
    assert_eq!(inner.token_tracker.cumulative_billed_output, 2900);
    assert_eq!(inner.token_tracker.reasoning_tokens, 1100);
    assert!(inner.restored_replay.is_some());
    assert_eq!(
        checkpoint
            .history
            .snapshot()?
            .ok_or("installed checkpoint")?
            .generation,
        1
    );
    Ok(())
}

fn retained_checkpoint(failure: &str, model: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut retained = json!({"version":1,"generation":1,"protocol":"full-current-dag",
        "model":model,"items":[]});
    match failure {
        "unsupported version" => retained["version"] = json!(2),
        "unsupported protocol" => retained["protocol"] = json!("native-memory"),
        "null checkpoint" => retained = Value::Null,
        "future generation" => retained["generation"] = json!(2),
        _ => return Err("unknown checkpoint failure".into()),
    }
    Ok(retained)
}

async fn state(
    background: &codelet_sessions::background_session::BackgroundSession,
    checkpoint: &CodexSession,
) -> Result<Value, Box<dyn std::error::Error>> {
    let inner = background.inner.lock().await;
    Ok(json!({
        "messages":inner.messages, "checkpoint":checkpoint.history.snapshot()?,
        "tokens":format!("{:?}", inner.token_tracker),
        "replay":format!("{:?}", inner.restored_replay),
        "output":background.get_buffered_output(100), "cachedTokens":background.get_tokens(),
        "snapshot":background.committed_context_snapshot(),
    }))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn unsupported_version_preserves_session_state() -> support::TestResult {
    // @step Given a fresh-process session with a committed generation and distinct existing runtime state
    // @step When persisted retained checkpoint metadata has <failure>
    // @step Then restore fails and session inference remains blocked
    // @step And messages checkpoint counters replay installation and output remain unchanged
    // @step And no physical request or completion is produced
    // @step When the authoritative valid replay is restored
    // @step Then the saved generation counters and history become usable
    exercise("unsupported version").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn unsupported_protocol_preserves_session_state() -> support::TestResult {
    // @step Given a fresh-process session with a committed generation and distinct existing runtime state
    // @step When persisted retained checkpoint metadata has <failure>
    // @step Then restore fails and session inference remains blocked
    // @step And messages checkpoint counters replay installation and output remain unchanged
    // @step And no physical request or completion is produced
    // @step When the authoritative valid replay is restored
    // @step Then the saved generation counters and history become usable
    exercise("unsupported protocol").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn null_checkpoint_preserves_session_state() -> support::TestResult {
    // @step Given a fresh-process session with a committed generation and distinct existing runtime state
    // @step When persisted retained checkpoint metadata has <failure>
    // @step Then restore fails and session inference remains blocked
    // @step And messages checkpoint counters replay installation and output remain unchanged
    // @step And no physical request or completion is produced
    // @step When the authoritative valid replay is restored
    // @step Then the saved generation counters and history become usable
    exercise("null checkpoint").await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn future_generation_preserves_session_state() -> support::TestResult {
    // @step Given a fresh-process session with a committed generation and distinct existing runtime state
    // @step When persisted retained checkpoint metadata has <failure>
    // @step Then restore fails and session inference remains blocked
    // @step And messages checkpoint counters replay installation and output remain unchanged
    // @step And no physical request or completion is produced
    // @step When the authoritative valid replay is restored
    // @step Then the saved generation counters and history become usable
    exercise("future generation").await
}
