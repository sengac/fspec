//! Feature: spec/features/structured-generation-aware-session-replay.feature

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::persistence::*;
use codelet_core::session_manager_handle::SessionManagerHandle;
use serde_json::{json, Value};
use support::{cold_resume, envelope, manager, next_anthropic_wire, TestResult};

fn append(session: &mut SessionManifest, role: &str, content: Value) -> Result<(), String> {
    append_message_with_metadata(
        session,
        role,
        "search preview",
        [("message".into(), json!({"role":role,"content":content}))]
            .into_iter()
            .collect(),
    )?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn structured_assistant_and_tool_history_survives_on_the_next_wire() -> TestResult {
    // @step Given persisted assistant strings signed thinking tool calls and tool results
    let (_directory, manager, session_id, baseline) = manager().await?;
    let mut session = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    append(
        &mut session,
        "assistant",
        json!("assistant string survives"),
    )?;
    append(
        &mut session,
        "assistant",
        json!([
            {"type":"thinking","thinking":"saved reasoning","signature":"saved-signature"},
            {"type":"tool_use","id":"call_saved","name":"lookup","input":{"query":"original"}}
        ]),
    )?;
    append(
        &mut session,
        "user",
        json!([
            {"type":"tool_result","tool_use_id":"call_saved","content":"complete tool result"},
            {"type":"text","text":"mixed user text"}
        ]),
    )?;
    drop(manager);
    // @step When cold restore prepares the next provider request
    let manager = cold_resume(&session_id)?;
    let wire = next_anthropic_wire(&manager, &session_id, baseline).await?;
    // @step Then the request preserves text thinking signature call identity and results
    let encoded = wire.to_string();
    for expected in [
        "assistant string survives",
        "saved reasoning",
        "saved-signature",
        "call_saved",
        "lookup",
        "original",
        "complete tool result",
        "mixed user text",
    ] {
        assert!(encoded.contains(expected), "lost {expected}: {wire}");
    }
    assert_eq!(encoded.matches("call_saved").count(), 2, "{wire}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn repeated_restore_replaces_rather_than_duplicates_history() -> TestResult {
    // @step Given a populated session with a committed summary and retained suffix
    let (_directory, manager, session_id, baseline) = manager().await?;
    let mut session = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    append(&mut session, "user", json!("covered original"))?;
    set_compaction_state(&mut session, "unique repeated DAG".into(), 1)?;
    append(&mut session, "user", json!("unique retained suffix"))?;
    let envelopes = get_session_message_envelopes(session.id)?;
    // @step When the same committed history is restored twice
    manager.restore_session_messages(&session_id, envelopes.clone())?;
    manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .messages
        .push(rig::message::Message::user("runtime after installation"));
    let installed = manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .messages
        .clone();
    manager.restore_session_messages(&session_id, envelopes)?;
    let repeated = manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .messages
        .clone();
    // @step Then one summary and one suffix remain with unchanged runtime context
    assert_eq!(installed, repeated, "second restore duplicated history");
    let wire = next_anthropic_wire(&manager, &session_id, baseline).await?;
    assert_eq!(wire.to_string().matches("unique repeated DAG").count(), 1);
    assert_eq!(
        wire.to_string().matches("unique retained suffix").count(),
        1
    );
    assert_eq!(
        wire.to_string()
            .matches("runtime after installation")
            .count(),
        1
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn unsupported_replay_blocks_fail_before_any_history_mutation() -> TestResult {
    // @step Given a valid prefix followed by an unsupported persisted content block
    let (_directory, manager, session_id, baseline) = manager().await?;
    let mut chunks = manager.chunks_tx().subscribe();
    let invalid = envelope(json!([{"type":"future_memory","payload":"unsupported"}]));
    // @step When structured restore is attempted
    let result = manager.restore_session_messages(
        &session_id,
        vec![
            envelope(json!("must not install partial prefix")).to_string(),
            invalid.to_string(),
        ],
    );
    // @step Then restore fails without installing the prefix or inventing replacement content
    assert!(
        result.is_err(),
        "unsupported replay must not silently disappear"
    );
    assert_eq!(
        manager
            .get_session(&session_id.value)?
            .inner
            .lock()
            .await
            .messages
            .len(),
        baseline
    );
    assert!(
        chunks.try_recv().is_err(),
        "invalid restore must not emit partial UI replay"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn a_stale_replay_revision_cannot_replace_a_newer_committed_history() -> TestResult {
    // @step Given replay envelopes captured before a newer message commit
    let (_directory, manager, session_id, _baseline) = manager().await?;
    let mut session = load_session(uuid::Uuid::parse_str(&session_id.value)?)?;
    append(&mut session, "user", json!("old snapshot"))?;
    let stale = get_session_message_envelopes(session.id)?;
    append(&mut session, "user", json!("new committed suffix"))?;
    manager.restore_session_messages(&session_id, get_session_message_envelopes(session.id)?)?;
    let installed = manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .messages
        .clone();
    // @step When the older snapshot is restored
    let result = manager.restore_session_messages(&session_id, stale);
    // @step Then restore reports a revision conflict and retains the installed history
    assert!(result.is_err(), "stale replay must conflict");
    assert_eq!(
        manager
            .get_session(&session_id.value)?
            .inner
            .lock()
            .await
            .messages,
        installed
    );
    Ok(())
}
