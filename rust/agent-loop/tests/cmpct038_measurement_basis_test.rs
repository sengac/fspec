#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/compaction-complete-measurement-basis.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios.
//
// CMPCT-038: CompactionComplete must carry compacted_tokens measured from the
// recalculated post-injection token tracker (reminders + summary), NOT from
// the wrapped DAG summary text alone. The emission therefore moves to the
// apply site via `apply_pending_dag_and_emit`.
//
// Exercises `codelet_agent_loop::inject_summary_handler`. Scenarios that
// required a raw pending DAG to be applied were dropped once the legacy
// apply path became fail-closed (see uncaptured_compaction_contract.rs and
// cmpct068_runtime_publication.rs); the clamp guard on the emit helper and
// the no-pending-DAG no-op remain covered here.

use std::sync::{Arc, Mutex};

use codelet_agent_loop::inject_summary_handler::{
    apply_pending_dag_and_emit, emit_post_injection_events,
};
use codelet_rpc_types::{CompactionResult, StreamChunk};

// ========================================
// Test helpers
// ========================================

fn create_test_session() -> codelet_cli::session::Session {
    let provider_manager = codelet_providers::ProviderManager::for_testing(
        codelet_providers::ProviderType::OpenAI,
        Some(200_000),
        Some(4096),
    );
    codelet_cli::session::Session::from_provider_manager(provider_manager)
}

fn push_user_message(session: &mut codelet_cli::session::Session, text: &str) {
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(text)),
    });
}

type Recorded = Arc<Mutex<Vec<StreamChunk>>>;

fn recorder() -> (Recorded, impl Fn(StreamChunk)) {
    let chunks: Recorded = Arc::new(Mutex::new(Vec::new()));
    let sink = chunks.clone();
    (chunks, move |chunk: StreamChunk| {
        sink.lock().unwrap().push(chunk);
    })
}

fn find_compaction_result(chunks: &[StreamChunk]) -> Option<CompactionResult> {
    chunks.iter().find_map(|c| match c {
        StreamChunk::CompactionComplete { compaction_result } => Some(compaction_result.clone()),
        _ => None,
    })
}

/// Unit-level clamp guard on the emit helper itself: compacted > original
/// must never produce a negative ratio.
#[test]
fn test_emit_post_injection_events_clamps_negative_ratio() {
    // @step Given a tiny session where surviving reminders plus summary exceed the original token count
    let (chunks, emit) = recorder();

    // @step When the agent loop applies the pending DAG and emits the completion chunk
    emit_post_injection_events(
        &emit, 1_000, // original
        1_500, // compacted (post-injection dominated by reminders)
    );

    // @step Then the emitted compression_ratio is exactly 0.0
    // @step And the compression_ratio is never negative on the wire
    let chunks = chunks.lock().unwrap();
    let result = find_compaction_result(&chunks).expect("CompactionComplete must be emitted");
    assert!(
        result.compression_ratio >= 0.0,
        "ratio must never be negative, got {}",
        result.compression_ratio
    );
    assert_eq!(
        result.compression_ratio, 0.0,
        "ratio must clamp to exactly 0.0"
    );
}

// ========================================
// Scenario: Running is emitted before CompactionComplete at the apply site
// ========================================

/// emit_post_injection_events emits SessionStateChange(Running) before
/// CompactionComplete and nothing else, so the TUI never flickers to Idle.
#[test]
fn test_emit_post_injection_events_emits_running_before_completion() {
    // @step Given known original and compacted token counts
    let (chunks, emit) = recorder();

    // @step When the completion events are emitted for those counts
    emit_post_injection_events(
        &emit, 40_000, // original
        10_000, // compacted
    );

    // @step Then a SessionStateChange Running chunk is emitted before the CompactionComplete chunk
    // @step And no Idle or Done chunk is emitted by the apply-and-emit step itself
    let chunks = chunks.lock().unwrap();
    assert_eq!(chunks.len(), 2);
    match &chunks[0] {
        StreamChunk::SessionStateChange { state } => {
            assert_eq!(*state, codelet_rpc_types::SessionState::Running);
        }
        other => panic!("First emission must be SessionStateChange(Running), got: {other:?}"),
    }
    match &chunks[1] {
        StreamChunk::CompactionComplete { compaction_result } => {
            assert_eq!(compaction_result.original_tokens, 40_000);
            assert_eq!(compaction_result.compacted_tokens, 10_000);
            assert!(
                compaction_result.compression_ratio > 0.0,
                "Compression ratio should be positive, got: {}",
                compaction_result.compression_ratio
            );
        }
        other => panic!("Second emission must be CompactionComplete, got: {other:?}"),
    }
}

// ========================================
// Scenario: Applying with no pending DAG emits nothing
// ========================================

#[test]
fn test_no_pending_dag_emits_nothing() {
    // @step Given a session with no pending DAG content
    let mut session = create_test_session();
    push_user_message(&mut session, "hello");
    let pending_dag: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

    // @step When the agent loop runs the apply-and-emit step
    let (chunks, emit) = recorder();
    let applied = apply_pending_dag_and_emit(&mut session, &pending_dag, 10_000, &emit);

    // @step Then no chunk is emitted
    assert!(
        chunks.lock().unwrap().is_empty(),
        "no chunks may be emitted when there is no pending DAG"
    );

    // @step And the step reports that nothing was applied
    assert!(
        applied.is_none(),
        "must return None when nothing was applied"
    );
}
