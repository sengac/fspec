use codelet_cli::interactive::{execute_compaction_and_capture_events, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_core::TokenState;
use codelet_providers::{ProviderManager, ProviderType};
use rig::message::Message;
use std::sync::{atomic::AtomicBool, Arc, Mutex};

struct Output;

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        assert!(!matches!(event, StreamEvent::CompactionComplete(_)));
    }
}

#[tokio::test]
#[serial_test::serial]
async fn recovery_preparation_preserves_the_authoritative_conversation(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("recovery", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "original question")?;
    append_message(&mut manifest, "assistant", "original evidence")?;
    let mut session = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::OpenAI,
        Some(200_000),
        Some(4096),
    ));
    session.messages = vec![
        Message::user("original question"),
        Message::assistant("original evidence"),
    ];
    session.turns = codelet_cli::interactive_helpers::convert_messages_to_turns(&session.messages);
    session.token_tracker.input_tokens = 120_000;
    session.token_tracker.cumulative_billed_input = 150_000;
    let messages = serde_json::to_value(&session.messages)?;
    let turns = session.turns.len();
    let durable = serde_json::to_value(load_session(manifest.id)?)?;
    let state = Arc::new(Mutex::new(TokenState {
        compaction_needed: true,
        ..TokenState::default()
    }));
    // @step Given a durable session with original conversation turns and provider state
    // @step When overflow recovery prepares an in-view DAG construction request
    execute_compaction_and_capture_events(
        &mut session,
        Arc::new(AtomicBool::new(false)),
        "resume original work",
        160_000,
        200_000,
        &state,
        &Output,
    )
    .await?;
    // @step Then original messages turns token counters and durable envelopes are unchanged
    assert_eq!(serde_json::to_value(&session.messages)?, messages);
    assert_eq!(session.turns.len(), turns);
    assert_eq!(session.token_tracker.input_tokens, 120_000);
    assert_eq!(session.token_tracker.cumulative_billed_input, 150_000);
    assert_eq!(serde_json::to_value(load_session(manifest.id)?)?, durable);
    // @step And ordinary inference stays blocked until a candidate is durably applied
    assert!(session.ensure_compaction_inference_ready().is_err());
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn recovery_preparation_does_not_persist_an_empty_codex_checkpoint(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a durable Codex session with a bound provider persistence writer
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("codex recovery", directory.path(), "codex")?;
    append_message(&mut manifest, "user", "original evidence")?;
    let mut session = Session::from_provider_manager(ProviderManager::for_testing(
        ProviderType::Codex,
        Some(200_000),
        Some(4096),
    ));
    session.bind_provider_persistence(manifest.id)?;
    session.messages = vec![Message::user("original evidence")];
    let durable = serde_json::to_value(load_session(manifest.id)?)?;
    let state = Arc::new(Mutex::new(TokenState::default()));
    // @step When overflow recovery prepares an in-view DAG construction request
    execute_compaction_and_capture_events(
        &mut session,
        Arc::new(AtomicBool::new(false)),
        "resume original work",
        160_000,
        200_000,
        &state,
        &Output,
    )
    .await?;
    // @step Then the persisted provider checkpoint and original envelopes remain unchanged
    assert_eq!(
        serde_json::to_value(load_session(manifest.id)?)?,
        durable,
        "builder preparation must not advance the persisted Codex checkpoint"
    );
    Ok(())
}
