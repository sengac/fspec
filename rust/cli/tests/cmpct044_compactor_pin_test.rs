//! Feature: spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature
//! Successful compaction uses selected-request admission, durable promotion and publication.
//! These eight legacy CMPCT-044 cases are retained and individually migrated to the
//! current CMPCT-079/057 contract (runtime-compaction-no-substitute-success.feature):
//! no partial, generic or alternate-model fallback.

use std::sync::{Arc, Mutex};

use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::system_reminders::is_system_reminder;
use codelet_cli::session::{Session, SystemReminderType};
use codelet_core::compaction::ConversationTurn;
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

/// The text of a user message, if it is one.
fn last_user_text_of(message: &Message) -> Option<String> {
    let Message::User { content } = message else {
        return None;
    };
    content.iter().find_map(|part| match part {
        UserContent::Text(text) => Some(text.text.clone()),
        _ => None,
    })
}

/// The last message text of a session (errors if empty or not a User).
fn last_user_text(session: &Session) -> TestResult<String> {
    let last = session.messages.last().ok_or("a message must remain")?;
    last_user_text_of(last).ok_or_else(|| "the pinned message must be a user text message".into())
}

/// Scenario: Successful sub-agent DAG is pinned to the parent session
#[tokio::test]
#[serial_test::serial]
async fn successful_sub_agent_dag_is_pinned_to_the_parent_session() -> TestResult {
    // @step Given a parent session with system reminders and many compactable conversation messages
    let mut fixture = Fixture::new(8)?;
    fixture
        .session
        .add_system_reminder(SystemReminderType::Environment, "platform: linux");
    let reminder = fixture
        .session
        .messages
        .last()
        .cloned()
        .ok_or("reminder appended")?;
    fixture.session.turns.push(ConversationTurn {
        user_message: "user turn 7 do some work".into(),
        tool_calls: Vec::new(),
        tool_results: Vec::new(),
        assistant_response: "assistant turn 7 did the work".into(),
        tokens: 10,
        timestamp: std::time::SystemTime::now(),
        previous_error: None,
    });
    let pre_pin_messages = fixture.session.messages.len();
    // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    fixture.valid_builder();
    let output = RecordingOutput::new();
    // @step When the trigger site pins the sub-agent's DAG to the parent
    fixture.run(&output).await?;
    let session = &fixture.session;
    let text = last_user_text(session)?;
    // @step Then the parent's in-memory messages are replaced by the retained system reminders plus the committed compaction summary
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
    assert_eq!(
        session
            .messages
            .iter()
            .filter(|message| serde_json::to_value(message).ok()
                == serde_json::to_value(&reminder).ok())
            .count(),
        1,
        "the system reminder must be retained exactly once"
    );
    assert!(
        session
            .messages
            .iter()
            .all(|message| is_system_reminder(message)
                || last_user_text_of(message).is_some_and(|text| text.contains("Overflow arc"))),
        "only reminders and the summary remain"
    );
    // @step And the parent's turn list is cleared
    assert!(
        session.turns.is_empty(),
        "the pin must clear the parent's turn list"
    );
    // @step And the parent's token tracker is recalculated from the reduced message list
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
    // @step Given a parent session whose turns are persisted in the session manifest
    let mut fixture = Fixture::new(6)?;
    let pre_pin = fixture.session.messages.clone();
    let original_ids = fixture.originals()?;
    let original_content = original_ids
        .iter()
        .map(|id| codelet_core::persistence::get_message(*id))
        .collect::<Result<Vec<_>, _>>()?;
    fixture.valid_builder();
    let output = RecordingOutput::new();
    // @step When the trigger site pins the sub-agent's DAG to the parent
    fixture.run(&output).await?;
    let session = &fixture.session;
    // @step Then the in-memory context is reduced to reminders plus the DAG
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
    // @step And the DAG is persisted once as a structurally identified compaction summary
    fixture.assert_typed_summary()?;
    // @step And the persisted session manifest history is NOT truncated
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

/// Scenario: Sub-agent timeout preserves history without a fallback DAG
#[tokio::test]
#[serial_test::serial]
async fn sub_agent_timeout_preserves_history_without_fallback() -> TestResult {
    // @step Given a parent session that is over its context limit
    let mut fixture = Fixture::new(10)?;
    let before = fixture.snapshot()?;
    // @step And the compactor sub-agent exceeds its wall-clock timeout without returning a DAG
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let observed = calls.clone();
    fixture.handler(Arc::new(move |_, _| {
        observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async { Err("compactor sub-agent timed out after 600s".into()) })
    }));
    let output = RecordingOutput::new();
    // @step When the trigger site attempts compaction
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("timeout cannot return fallback success")?;
    // @step Then compaction fails without pinning any fallback DAG and without a further LLM call
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
    // @step And the parent's in-memory and committed history are unchanged
    fixture.assert_memory_preserved(&before)?;
    // @step And exactly one CompactionFailed event carries the timeout reason and no CompactionComplete is emitted
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

/// Scenario: Unparseable sub-agent output preserves history without a fallback DAG
#[tokio::test]
#[serial_test::serial]
async fn unparseable_sub_agent_output_preserves_history_without_fallback() -> TestResult {
    let mut fixture = Fixture::new(5)?;
    let before = fixture.snapshot()?;
    // @step Given the compactor sub-agent returns final text containing no parseable dag-node blocks
    fixture.handler(Arc::new(|_, _| {
        Box::pin(async { Ok("sorry, no parseable DAG here".into()) })
    }));
    let output = RecordingOutput::new();
    // @step When the trigger site attempts compaction
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("invalid output cannot return substitute success")?;
    // @step Then the attempt fails as an invalid candidate and no fallback DAG is pinned
    assert!(matches!(
        error,
        codelet_cli::compaction_operation::CompactionFailure::Candidate(_)
    ));
    // @step And the parent's in-memory and committed history are unchanged
    fixture.assert_memory_preserved(&before)?;
    // @step And exactly one CompactionFailed event is emitted and no CompactionComplete is emitted
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

/// Scenario: Partial dag-node blocks from a failed sub-agent are never installed
#[tokio::test]
#[serial_test::serial]
async fn partial_dag_diagnostics_are_never_installed_as_fallback() -> TestResult {
    let mut fixture = Fixture::new(4)?;
    let before = fixture.snapshot()?;
    // @step Given the compactor sub-agent times out after emitting some complete dag-node blocks but no final DAG
    const PARTIAL: &str = "surveying the session...\n<dag-node depth=\"D1\" turns=\"0-3\" label=\"Recovered arc\">recovered work survived the timeout</dag-node>\n...truncated mid-survey";
    fixture.handler(Arc::new(|_, _| Box::pin(async { Err(PARTIAL.into()) })));
    let output = RecordingOutput::new();
    // @step When the trigger site attempts compaction
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("partial diagnostics cannot authorize fallback")?;
    // @step Then the partial dag-node blocks are reported only as failure diagnostics and never installed as a DAG
    assert!(matches!(
        &error,
        codelet_cli::compaction_operation::CompactionFailure::Builder(_)
    ));
    assert!(error.to_string().contains("Recovered arc"));
    assert!(error
        .to_string()
        .contains("recovered work survived the timeout"));
    // @step And the parent's in-memory and committed history are unchanged
    fixture.assert_memory_preserved(&before)?;
    // @step And exactly one CompactionFailed event carries the builder diagnostics and no CompactionComplete is emitted
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
    // @step Given the compactor sub-agent run begins for an over-limit parent session
    let mut fixture = Fixture::new(3)?;
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observation = observed.clone();
    let flag = fixture.busy.clone();
    let dag = fixture.dag();
    // @step When the sub-agent runs and reads the parent via SessionSearch
    // (the builder handler stands in for the sub-agent run and observes the flag)
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
    // @step Then the compaction_in_progress flag is true during the sub-agent run so Layer-0 trimming applies to its reads
    assert!(
        observed.load(std::sync::atomic::Ordering::Acquire),
        "the actual busy flag must be true inside the builder"
    );
    // @step And the compaction_in_progress flag is false after the pin completes
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
    // @step Given an overflow error triggers the compactor sub-agent for a session
    let mut fixture = Fixture::new(6)?;
    let output = Arc::new(RecordingOutput::new());
    let seen_by_builder = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (observer, seen, dag) = (output.clone(), seen_by_builder.clone(), fixture.dag());
    fixture.handler(Arc::new(move |_, _| {
        // The builder runs as the spawned sub-agent: record whether the
        // start event was already emitted when it was spawned.
        let started = observer.count(|event| matches!(event, StreamEvent::CompactionStarted)) == 1;
        seen.store(started, std::sync::atomic::Ordering::Release);
        let dag = dag.clone();
        Box::pin(async move { Ok(dag) })
    }));
    // @step When the sub-agent run and pin complete
    fixture.run(&*output).await?;
    // @step Then a compaction-started event was emitted before the sub-agent was spawned
    assert!(
        seen_by_builder.load(std::sync::atomic::Ordering::Acquire),
        "CompactionStarted must be emitted before the builder runs"
    );
    // @step And a compaction-complete event is emitted after the pin with the recalculated post-pin token basis
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
    // @step And the CompactionComplete event reflects the reduced context, not just the DAG summary size
    let boundary = codelet_core::persistence::load_session(fixture.id)?
        .compaction_commit
        .ok_or("committed boundary missing")?;
    assert!(!boundary.publication_pending);
    assert_eq!(u64::from(compacted), boundary.input_tokens);
    Ok(())
}

/// Scenario: Missing compactor handler fails without a fallback DAG
#[tokio::test]
#[serial_test::serial]
async fn missing_compactor_handler_preserves_history_without_fallback() -> TestResult {
    // @step Given a session for which no compactor-sub-agent handler is registered
    // @step And the session is over its context limit with a terminal overflow error
    let mut fixture = Fixture::new(7)?;
    assert!(
        !codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler(fixture.id),
        "precondition: no handler registered"
    );
    let before = fixture.snapshot()?;
    let output = RecordingOutput::new();
    // @step When the trigger site attempts compactor recovery
    let error = fixture
        .run(&output)
        .await
        .err()
        .ok_or("missing builder cannot return fallback success")?;
    // @step Then compaction fails as unavailable and no fallback DAG is injected
    assert!(matches!(
        error,
        codelet_cli::compaction_operation::CompactionFailure::Unavailable
    ));
    // @step And the session's in-memory and committed history are unchanged
    fixture.assert_memory_preserved(&before)?;
    // @step And the failure is surfaced with a structured compaction-failed lifecycle event
    let (failed, complete) = output.compaction_failed_then_complete_positions();
    assert!(
        failed.is_some(),
        "a structured failure must surface the missing builder"
    );
    assert!(
        complete.is_none(),
        "missing builder must not be followed by successful completion"
    );
    // @step And no CompactionComplete event is emitted
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
