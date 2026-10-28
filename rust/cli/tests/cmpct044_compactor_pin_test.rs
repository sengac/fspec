//! Feature: spec/features/runtime-compaction-no-substitute-success.feature
//! Successful compaction uses selected-request admission, durable promotion and publication.
//! These eight legacy CMPCT-044 cases are retained and individually migrated to the
//! current CMPCT-079/057 contract: no partial, generic or alternate-model fallback.

use std::sync::{Arc, Mutex};

use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use rig::message::{Message, UserContent};
#[path = "fixtures/compactor_contract.rs"]
pub mod contract;
use contract::{Fixture, TestResult};

/// Fake StreamOutput that records every emitted event.
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    fn count<F: Fn(&StreamEvent) -> bool>(&self, pred: F) -> usize {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|e| pred(e))
            .count()
    }

    /// The (position) of the first `CompactionFailed` and the first
    /// `CompactionComplete` — both `Some` iff both were emitted, in which
    /// case the ordering assertion compares them directly.
    fn compaction_failed_then_complete_positions(&self) -> (Option<usize>, Option<usize>) {
        let events = self
            .events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let failed = events
            .iter()
            .position(|e| matches!(e, StreamEvent::CompactionFailed { .. }));
        let complete = events
            .iter()
            .position(|e| matches!(e, StreamEvent::CompactionComplete { .. }));
        (failed, complete)
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    }
}

/// The last message text of a session (panics if empty or not a User).
fn last_user_text(session: &Session) -> TestResult<String> {
    let last = session.messages.last().ok_or("a message must remain")?;
    let Message::User { content } = last else {
        return Err("the pinned message must be a user message".into());
    };
    content
        .iter()
        .find_map(|part| match part {
            UserContent::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .ok_or_else(|| "the pinned user message must contain text".into())
}

/// Scenario: Successful sub-agent DAG is pinned to the parent session
#[tokio::test]
#[serial_test::serial]
async fn successful_sub_agent_dag_is_pinned_to_the_parent_session() -> TestResult {
    let mut fixture = Fixture::new(8)?;
    let pre_pin_messages = fixture.session.messages.len();
    let pre_pin_turns = fixture.session.turns.len();
    fixture.valid_builder();
    let output = RecordingOutput::new();
    fixture.run(&output).await?;
    let session = &fixture.session;
    let text = last_user_text(session)?;
    fixture.assert_typed_summary()?;
    assert!(
        text.contains("Overflow arc"),
        "the pinned DAG must be the sub-agent's DAG, got: {text}"
    );
    assert!(
        session.messages.len() < pre_pin_messages,
        "in-memory messages must be reduced ({} -> {})",
        pre_pin_messages,
        session.messages.len()
    );
    assert!(
        session.turns.is_empty() || pre_pin_turns >= session.turns.len(),
        "the pin must clear the parent's turn list"
    );
    assert!(
        session.token_tracker.input_tokens < 190_000,
        "the tracker must reflect the reduced context, got {}",
        session.token_tracker.input_tokens
    );
    let manifest = codelet_core::persistence::load_session(fixture.id)?;
    assert_eq!(
        manifest.compaction.ok_or("durable DAG missing")?.summary,
        fixture.dag()
    );
    let committed = manifest
        .compaction_commit
        .ok_or("committed identity missing")?;
    assert_eq!(committed.identity.generation, 1);
    assert!(!committed.publication_pending);
    session.ensure_compaction_inference_ready()?;
    Ok(())
}

/// Scenario: Pinning a sub-agent DAG keeps the persisted history intact
#[tokio::test]
#[serial_test::serial]
async fn pinning_keeps_the_persisted_history_intact() -> TestResult {
    let mut fixture = Fixture::new(6)?;
    let pre_pin = fixture.session.messages.clone();
    let original_ids = fixture.originals()?;
    let original_content = original_ids
        .iter()
        .map(|id| codelet_core::persistence::get_message(*id))
        .collect::<Result<Vec<_>, _>>()?;
    fixture.valid_builder();
    let output = RecordingOutput::new();
    fixture.run(&output).await?;
    let session = &fixture.session;
    assert!(
        session.messages.len() < pre_pin.len(),
        "in-memory context must be reduced ({} -> {})",
        pre_pin.len(),
        session.messages.len()
    );
    let text = last_user_text(session)?;
    assert!(
        text.contains("Overflow arc"),
        "the reduced context must carry the sub-agent's DAG"
    );
    assert!(
        !text.contains("user turn 0 do some work"),
        "the pinned message is the DAG, not a replay of pre-pin history"
    );
    fixture.assert_typed_summary()?;
    assert_eq!(fixture.originals()?, original_ids);
    let restored_content = original_ids
        .iter()
        .map(|id| codelet_core::persistence::get_message(*id))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        serde_json::to_value(restored_content)?,
        serde_json::to_value(original_content)?
    );
    Ok(())
}

/// Scenario: Sub-agent timeout pins a free fallback DAG
#[tokio::test]
#[serial_test::serial]
async fn sub_agent_timeout_preserves_history_without_fallback() -> TestResult {
    let mut fixture = Fixture::new(10)?;
    let before = fixture.snapshot()?;
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    fixture.handler(Arc::new(move |_, _| {
        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async { Err("compactor sub-agent timed out after 600s".into()) })
    }));
    let output = RecordingOutput::new();
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("timeout cannot return fallback success")?;
    assert!(matches!(
        &error,
        codelet_cli::compaction_operation::CompactionFailure::Builder(_)
    ));
    assert!(error.to_string().contains("timed out"));
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "no further LLM call after timeout"
    );
    fixture.assert_memory_preserved(&before)?;
    let (failed_at, complete_at) = output.compaction_failed_then_complete_positions();
    assert!(
        failed_at.is_some(),
        "timeout must surface a structured failure"
    );
    assert!(
        complete_at.is_none(),
        "failed compaction must not report completion"
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        1
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    let events = output.events.lock().map_err(|error| error.to_string())?;
    assert!(events.iter().any(|event|matches!(event,StreamEvent::CompactionFailed{reason} if reason.contains("timed out"))));
    Ok(())
}

/// Scenario: Unparseable sub-agent output pins a fallback DAG
#[tokio::test]
#[serial_test::serial]
async fn unparseable_sub_agent_output_preserves_history_without_fallback() -> TestResult {
    let mut fixture = Fixture::new(5)?;
    let before = fixture.snapshot()?;
    fixture.handler(Arc::new(|_, _| {
        Box::pin(async { Ok("sorry, no parseable DAG here".into()) })
    }));
    let output = RecordingOutput::new();
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("invalid output cannot return substitute success")?;
    assert!(matches!(
        error,
        codelet_cli::compaction_operation::CompactionFailure::Candidate(_)
    ));
    fixture.assert_memory_preserved(&before)?;
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        1
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    let (failed, complete) = output.compaction_failed_then_complete_positions();
    assert!(failed.is_some());
    assert!(complete.is_none());
    Ok(())
}

/// Scenario: Partial dag-node blocks from a failed sub-agent are recovered
#[tokio::test]
#[serial_test::serial]
async fn partial_dag_diagnostics_are_never_installed_as_fallback() -> TestResult {
    let mut fixture = Fixture::new(4)?;
    let before = fixture.snapshot()?;
    const PARTIAL: &str = "surveying the session...\n<dag-node depth=\"D1\" turns=\"0-3\" label=\"Recovered arc\">recovered work survived the timeout</dag-node>\n...truncated mid-survey";
    fixture.handler(Arc::new(|_, _| Box::pin(async { Err(PARTIAL.into()) })));
    let output = RecordingOutput::new();
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("partial diagnostics cannot authorize fallback")?;
    assert!(matches!(
        &error,
        codelet_cli::compaction_operation::CompactionFailure::Builder(_)
    ));
    assert!(error.to_string().contains("Recovered arc"));
    assert!(error
        .to_string()
        .contains("recovered work survived the timeout"));
    fixture.assert_memory_preserved(&before)?;
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        1
    );
    let (failed, complete) = output.compaction_failed_then_complete_positions();
    assert!(failed.is_some());
    assert!(
        complete.is_none(),
        "diagnostics may not be followed by successful completion"
    );
    assert!(!serde_json::to_string(&fixture.session.messages)?.contains("Recovered arc"));
    let events = output.events.lock().map_err(|error| error.to_string())?;
    assert!(events.iter().any(|event|matches!(event,StreamEvent::CompactionFailed{reason} if reason.contains("compaction builder failed") && reason.contains("Recovered arc"))));
    Ok(())
}

/// Scenario: compaction_in_progress gates sub-agent reads and is cleared after the pin
#[tokio::test]
#[serial_test::serial]
async fn compaction_flag_true_during_run_and_false_after_pin() -> TestResult {
    let mut fixture = Fixture::new(3)?;
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observation = observed.clone();
    let flag = fixture.busy.clone();
    let dag = fixture.dag();
    fixture.handler(Arc::new(move |_, _| {
        observation.store(
            flag.load(std::sync::atomic::Ordering::Acquire),
            std::sync::atomic::Ordering::Release,
        );
        let dag = dag.clone();
        Box::pin(async move { Ok(dag) })
    }));
    let output = RecordingOutput::new();
    fixture.run(&output).await?;
    assert!(
        observed.load(std::sync::atomic::Ordering::Acquire),
        "the actual busy flag must be true inside the builder"
    );
    assert!(
        !fixture.busy.load(std::sync::atomic::Ordering::Acquire),
        "the flag must be cleared after the pin"
    );
    assert!(
        !fixture
            .tokens
            .lock()
            .map_err(|error| error.to_string())?
            .compaction_needed
    );
    Ok(())
}

/// Scenario: Compaction lifecycle events are emitted for the sub-agent path
#[tokio::test]
#[serial_test::serial]
async fn lifecycle_events_are_emitted_for_the_sub_agent_path() -> TestResult {
    let mut fixture = Fixture::new(6)?;
    fixture.valid_builder();
    let output = RecordingOutput::new();
    fixture.run(&output).await?;
    let progress = output.count(|event| matches!(event, StreamEvent::CompactionProgress(_)));
    assert!(
        progress >= 1,
        "a compaction progress update must be emitted (got {progress})"
    );
    let complete = output.count(|event| matches!(event, StreamEvent::CompactionComplete(_)));
    assert_eq!(
        complete, 1,
        "exactly one CompactionComplete must be emitted (got {complete})"
    );
    let events = output.events.lock().map_err(|error| error.to_string())?;
    let (original, compacted) = events
        .iter()
        .find_map(|event| match event {
            StreamEvent::CompactionComplete(info) => {
                Some((info.original_tokens, info.compacted_tokens))
            }
            _ => None,
        })
        .ok_or("CompactionComplete must carry the token basis")?;
    assert_eq!(
        original, 190_000,
        "the pre-compaction basis must be the shared token state's input basis"
    );
    assert!(
        compacted < original,
        "compacted_tokens must reflect the admitted reduced context"
    );
    let started = events
        .iter()
        .position(|event| matches!(event, StreamEvent::CompactionStarted))
        .ok_or("start missing")?;
    let published = events
        .iter()
        .position(|event| matches!(event, StreamEvent::ContextSnapshot(_)))
        .ok_or("committed publication missing")?;
    let completed = events
        .iter()
        .position(|event| matches!(event, StreamEvent::CompactionComplete(_)))
        .ok_or("completion missing")?;
    assert!(started < published && published < completed);
    assert!(!events
        .iter()
        .any(|event| matches!(event, StreamEvent::CompactionFailed { .. })));
    let boundary = codelet_core::persistence::load_session(fixture.id)?
        .compaction_commit
        .ok_or("committed boundary missing")?;
    assert!(!boundary.publication_pending);
    assert_eq!(u64::from(compacted), boundary.input_tokens);
    Ok(())
}

/// Scenario: Missing compactor handler still guarantees a reduced session
#[tokio::test]
#[serial_test::serial]
async fn missing_compactor_handler_preserves_history_without_fallback() -> TestResult {
    let mut fixture = Fixture::new(7)?;
    assert!(
        !codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler(fixture.id),
        "precondition: no handler registered"
    );
    let before = fixture.snapshot()?;
    let output = RecordingOutput::new();
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("missing builder cannot return fallback success")?;
    assert!(matches!(
        error,
        codelet_cli::compaction_operation::CompactionFailure::Unavailable
    ));
    fixture.assert_memory_preserved(&before)?;
    let (failed, complete) = output.compaction_failed_then_complete_positions();
    assert!(
        failed.is_some(),
        "a structured failure must surface the missing builder"
    );
    assert!(
        complete.is_none(),
        "missing builder must not be followed by successful completion"
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionComplete(_))),
        0
    );
    assert_eq!(
        output.count(|event| matches!(event, StreamEvent::CompactionFailed { .. })),
        1
    );
    let events = output.events.lock().map_err(|error| error.to_string())?;
    assert!(events.iter().any(|event|matches!(event,StreamEvent::CompactionFailed{reason} if reason.contains("unavailable"))));
    Ok(())
}
