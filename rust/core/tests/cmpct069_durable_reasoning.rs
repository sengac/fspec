//! Feature: spec/features/durable-reasoning-settlement.feature

use codelet_core::compaction_hook::{CompactionHook, TokenState};
use codelet_core::persistence::compaction_commit::{commit_compaction, reconcile_compaction};
use codelet_core::persistence::request_settlement::{replace_request_context, settle_request};
use codelet_core::persistence::*;
use codelet_core::request_settlement_hook::RequestSettlementHook;
use codelet_test_helpers::setup_temp_directory;
use rig::agent::{CancelSignal, StreamingPromptHook};
use rig::completion::Usage;
use rig::message::Message;
use rig::providers::openai::completion::CompletionModel;
use std::sync::{Arc, Mutex};

#[path = "support/compaction_boundary.rs"]
mod support;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn configure(path: &std::path::Path) -> Result<(), String> {
    codelet_common::set_data_directory(path.to_path_buf())?;
    reset_stores_for_tests();
    Ok(())
}

fn verify_restored(session: &SessionManifest) -> TestResult {
    assert_eq!(session.token_usage.reasoning_tokens, 48);
    assert_eq!(session.token_usage.cumulative_billed_input, 200);
    assert_eq!(session.token_usage.cumulative_billed_output, 177);
    assert_eq!(session.request_settlements.len(), 2);
    assert_eq!(
        session
            .compaction_commit
            .as_ref()
            .ok_or("missing boundary")?
            .identity
            .generation,
        1
    );
    assert!(!get_session_message_envelopes(session.id)?.is_empty());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn measured_terminal_reasoning_survives_a_compacted_cold_resume() -> TestResult {
    // @step Given a persisted session with earlier cumulative reasoning and measured terminal requests
    if let Ok(path) = std::env::var("CMPCT069_REASONING_STORE") {
        configure(std::path::Path::new(&path))?;
        let store = SessionStore::new()?;
        let session = store.list_all().first().ok_or("missing cold session")?.id;
        return verify_restored(&store.load(session)?);
    }
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut session = create_session_with_provider("reasoning", directory.path(), "openai")?;
    append_message(&mut session, "user", "immutable original")?;
    update_session_tokens(&mut session, 0, 0, 0, 0, 11)?;
    let hook = RequestSettlementHook::new(
        CompactionHook::new(Arc::new(Mutex::new(TokenState::default())), u64::MAX),
        session.id,
    );
    let prompt = Message::user("hello");
    // @step When duplicate terminal delivery context replacement and compaction are completed
    for reasoning in [30, 7] {
        StreamingPromptHook::<CompletionModel>::on_completion_call(
            &hook,
            &prompt,
            &[],
            CancelSignal::new(),
        )
        .await;
        let response = serde_json::from_value(serde_json::json!({"usage": {
            "prompt_tokens": 100, "completion_tokens": 70 + reasoning,
            "total_tokens": 170 + reasoning,
            "completion_tokens_details": {"reasoning_tokens": reasoning}
        }}))?;
        for _duplicate in 0..2 {
            let signal = CancelSignal::new();
            StreamingPromptHook::<CompletionModel>::on_stream_completion_response_finish(
                &hook,
                &prompt,
                &response,
                signal.clone(),
            )
            .await;
            assert!(!signal.is_cancelled());
            hook.check()?;
        }
    }
    replace_request_context(session.id, None)?;
    let current = load_session(session.id)?;
    assert_eq!(current.token_usage.current_context_tokens, 0);
    let promoted = commit_compaction(support::candidate(&current)?, "model-a", |_, _| Ok(()))?;
    let identity = &promoted
        .compaction_commit
        .as_ref()
        .ok_or("missing boundary")?
        .identity;
    let ready = reconcile_compaction(identity)?;
    // @step Then a fresh process restores every measured reasoning token and bills each request once
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "measured_terminal_reasoning_survives_a_compacted_cold_resume",
            "--nocapture",
        ])
        .env("CMPCT069_REASONING_STORE", directory.path())
        .output()?;
    assert!(
        child.status.success(),
        "cold reader failed: {}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    verify_restored(&ready)
}

#[test]
#[serial_test::serial]
fn reasoning_overflow_rejects_the_complete_settlement_atomically() -> TestResult {
    // @step Given a persisted session whose cumulative reasoning cannot accept another measured token
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut session = create_session("overflow", directory.path())?;
    update_session_tokens(&mut session, 0, 0, 0, 0, u64::MAX)?;
    let path = directory
        .path()
        .join(format!("sessions/{}.json", session.id));
    let before = std::fs::read(&path)?;
    let usage = Usage {
        input_tokens: 1,
        output_tokens: 1,
        reasoning_tokens: Some(1),
        total_tokens: 3,
        ..Default::default()
    };
    // @step When a new physical request attempts settlement
    let result = settle_request(session.id, "overflow-request", &usage);
    // @step Then settlement fails without changing durable bytes billing or physical identities
    assert!(result.is_err(), "reasoning overflow accepted: {result:?}");
    assert_eq!(std::fs::read(path)?, before);
    assert_eq!(load_session(session.id)?.token_usage, session.token_usage);
    assert!(load_session(session.id)?.request_settlements.is_empty());
    Ok(())
}

#[test]
#[serial_test::serial]
fn unknown_reasoning_does_not_erase_earlier_measured_reasoning() -> TestResult {
    // @step Given a persisted session with measured cumulative reasoning
    let directory = setup_temp_directory()?;
    configure(directory.path())?;
    let mut session = create_session("unknown reasoning", directory.path())?;
    update_session_tokens(&mut session, 0, 0, 0, 0, 9)?;
    let usage = Usage {
        input_tokens: 10,
        output_tokens: 5,
        total_tokens: 15,
        ..Default::default()
    };
    // @step When a new physical request settles without measured reasoning detail
    settle_request(session.id, "unknown-detail", &usage)?;
    replace_request_context(session.id, None)?;
    reset_stores_for_tests();
    // @step Then cold reload preserves earlier reasoning and settles only the measured spend
    let restored = load_session(session.id)?;
    assert_eq!(restored.token_usage.reasoning_tokens, 9);
    assert_eq!(restored.token_usage.cumulative_billed_output, 5);
    assert_eq!(
        restored.request_settlements["unknown-detail"].reasoning_tokens,
        None
    );
    Ok(())
}
