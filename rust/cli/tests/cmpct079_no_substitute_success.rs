// Feature: spec/features/runtime-compaction-no-substitute-success.feature
use codelet_cli::compaction_operation::CompactionFailure;
use codelet_cli::compactor_sub_agent::{
    run_compactor_sub_agent_round, set_compactor_sub_agent_handler, CompactorSubAgentHandler,
};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::TokenState;
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[derive(Default)]
struct RecordingOutput(Mutex<Vec<StreamEvent>>);

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

struct DropWitness(Arc<AtomicBool>);

impl Drop for DropWitness {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

async fn assert_blocked_failure(
    handler: Option<CompactorSubAgentHandler>,
    expected: CompactionFailure,
) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    codelet_core::persistence::reset_stores_for_tests();
    let mut manifest = codelet_core::persistence::create_session_with_provider(
        "blocked compaction",
        directory.path(),
        "openai/o3",
    )?;
    codelet_core::persistence::append_message(
        &mut manifest,
        "user",
        "committed suffix must remain intact",
    )?;
    let session_id = manifest.id;
    let committed = std::fs::read(
        directory
            .path()
            .join("sessions")
            .join(format!("{session_id}.json")),
    )?;
    let mut manager = codelet_providers::ProviderManager::for_testing(
        codelet_providers::ProviderType::OpenAI,
        Some(200_000),
        Some(4096),
    );
    manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("committed suffix must remain intact")),
    });
    session.token_tracker.input_tokens = 120_000;
    let original = serde_json::to_value(&session.messages)?;
    let state = Arc::new(Mutex::new(TokenState {
        input_tokens: 120_000,
        compaction_needed: true,
        ..TokenState::default()
    }));
    let busy = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::default();
    set_compactor_sub_agent_handler(session_id, handler);
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        run_compactor_sub_agent_round(&mut session, session_id, &state, &busy, &output),
    )
    .await;
    set_compactor_sub_agent_handler(session_id, None);
    let failure = result?
        .err()
        .ok_or("failed builder must not return substitute success")?;
    assert_eq!(
        std::mem::discriminant(&failure),
        std::mem::discriminant(&expected),
        "{failure}"
    );
    assert!(session.ensure_compaction_inference_ready().is_err());
    assert_eq!(serde_json::to_value(&session.messages)?, original);
    assert_eq!(
        std::fs::read(
            directory
                .path()
                .join("sessions")
                .join(format!("{session_id}.json"))
        )?,
        committed
    );
    assert_eq!(session.token_tracker.input_tokens, 120_000);
    assert!(!busy.load(Ordering::Acquire));
    assert!(
        state
            .lock()
            .map_err(|error| error.to_string())?
            .compaction_needed
    );
    let events = output.0.lock().map_err(|error| error.to_string())?;
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, StreamEvent::CompactionFailed { .. }))
            .count(),
        1
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionComplete { .. })));
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn missing_builder_preserves_the_committed_conversation() -> TestResult {
    // @step Given committed conversation and no registered DAG builder
    // @step When automatic compaction runs
    // @step Then a typed missing-builder failure is returned
    // @step And committed conversation is unchanged without completion
    // @step And the busy indicator clears while inference remains blocked
    assert_blocked_failure(None, CompactionFailure::Unavailable).await
}

#[tokio::test]
#[serial_test::serial]
async fn builder_diagnostics_cannot_become_a_replacement_dag() -> TestResult {
    // @step Given committed conversation and a builder failing with valid-looking DAG diagnostics
    // @step When automatic compaction runs
    // @step Then a typed builder failure is returned
    // @step And committed conversation is unchanged without completion
    // @step And the busy indicator clears while inference remains blocked
    assert_blocked_failure(Some(Arc::new(|_, _| Box::pin(async {
        Err("provider failed: <dag-node depth=\"D1\" turns=\"0-1\" label=\"fake\">not evidence</dag-node>".into())
    }))), CompactionFailure::Builder(String::new())).await
}

#[tokio::test]
#[serial_test::serial]
async fn invalid_builder_output_cannot_become_a_replacement_dag() -> TestResult {
    // @step Given committed conversation and a builder returning invalid output
    // @step When automatic compaction runs
    // @step Then a typed invalid-candidate failure is returned
    // @step And committed conversation is unchanged without completion
    // @step And the busy indicator clears while inference remains blocked
    assert_blocked_failure(
        Some(Arc::new(|_, _| {
            Box::pin(async { Ok("not a complete DAG".into()) })
        })),
        CompactionFailure::Candidate(String::new()),
    )
    .await
}

#[tokio::test]
#[serial_test::serial]
async fn deadline_cancels_a_stalled_builder_without_changing_committed_memory() -> TestResult {
    // @step Given committed conversation and a builder waiting indefinitely
    // @step When the declared compaction deadline expires
    // @step Then a typed deadline failure is returned within the harness deadline
    // @step And committed conversation is unchanged without completion
    // @step And the busy indicator clears while inference remains blocked
    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(Some(
        Duration::from_millis(20),
    ));
    let dropped = Arc::new(AtomicBool::new(false));
    let observed = dropped.clone();
    let result = assert_blocked_failure(
        Some(Arc::new(move |_, _| {
            let witness = DropWitness(observed.clone());
            Box::pin(async move {
                let _witness = witness;
                std::future::pending().await
            })
        })),
        CompactionFailure::Deadline,
    )
    .await;
    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);
    result?;
    assert!(dropped.load(Ordering::Acquire));
    Ok(())
}
