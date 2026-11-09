#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/compaction-status-lifecycle.feature
//!
//! COV-057: the status lifecycle around automatic compaction, observed on the
//! real agent loop. The compaction builder durably commits the DAG before
//! `CompactionComplete`; `CompactionContinuing` returns the session to
//! Running; turn reconciliation finds no second candidate and the turn ends
//! Idle.

#[path = "support/terminal_recovery.rs"]
pub mod terminal;

#[path = "cov057_compaction_status_lifecycle/live.rs"]
pub mod live;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::interactive::{StreamEvent, StreamOutput};
use codelet_rpc_types::{SessionState, SessionStatus, StreamChunk};
use live::{Live, Trigger, ANSWER, LABEL};

fn position(chunks: &[StreamChunk], from: usize, found: impl Fn(&StreamChunk) -> bool) -> usize {
    chunks
        .iter()
        .skip(from)
        .position(found)
        .map(|offset| from + offset)
        .unwrap_or_else(|| panic!("expected chunk after index {from} in {chunks:?}"))
}

fn state(chunk: &StreamChunk, expected: SessionState) -> bool {
    matches!(chunk, StreamChunk::SessionStateChange { state } if *state == expected)
}

fn is_completion(chunk: &StreamChunk) -> bool {
    matches!(chunk, StreamChunk::CompactionComplete { .. })
}

fn committed_generation(live: &Live) -> Option<u64> {
    codelet_core::persistence::load_session(live.session.id)
        .expect("durable manifest")
        .compaction_commit
        .map(|commit| commit.identity.generation)
}

/// Indices of the automatic compaction lifecycle inside one observed turn.
struct Lifecycle {
    compacting: usize,
    committed: usize,
    completion: usize,
    running: usize,
    answer: usize,
    idle: usize,
    done: usize,
}

impl Lifecycle {
    fn of(chunks: &[StreamChunk]) -> Self {
        let compacting = position(chunks, 0, |c| state(c, SessionState::Compacting));
        let committed = position(
            chunks,
            compacting,
            |c| matches!(c, StreamChunk::CommittedCompaction { snapshot } if snapshot.reconciled),
        );
        let completion = position(chunks, committed, is_completion);
        let running = position(chunks, completion, |c| state(c, SessionState::Running));
        let answer = position(
            chunks,
            running,
            |c| matches!(c, StreamChunk::Text { text, .. } if text.contains(ANSWER)),
        );
        let idle = position(chunks, answer, |c| state(c, SessionState::Idle));
        let done = position(chunks, answer, |c| matches!(c, StreamChunk::Done));
        Self {
            compacting,
            committed,
            completion,
            running,
            answer,
            idle,
            done,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_post_loop_compaction_keeps_status_active_through_dag_construction() {
    // @step Given an agent stream response has just finished above the compaction threshold
    let mut live = Live::new(Trigger::PostResponse).await;
    let chunks = live.turn().await;
    let lifecycle = Lifecycle::of(&chunks);
    let tool = position(&chunks, 0, |c| matches!(c, StreamChunk::ToolResult { .. }));
    assert!(
        tool < lifecycle.compacting,
        "the finished response's tool ran first"
    );
    // @step And the compaction hook detected that compaction is needed
    let bodies = live.bodies().await;
    assert!(
        bodies
            .get(1)
            .is_some_and(|body| body.contains(&live.target())),
        "the over-threshold follow-up request was cancelled before reaching the provider"
    );
    // @step When stream_loop emits CompactionStarted
    // @step Then the session status should be Compacting
    assert!(lifecycle.compacting > tool);
    // @step When the compaction builder durably commits the DAG
    assert_eq!(live.builders.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(committed_generation(&live), Some(1));
    // @step Then CompactionComplete should be emitted only after the durable commit
    assert!(!chunks[..lifecycle.committed].iter().any(is_completion));
    assert!(lifecycle.committed < lifecycle.completion);
    // @step When stream_loop emits CompactionContinuing
    // @step Then the agent-loop BackgroundOutput should set session status to Running
    assert!(lifecycle.completion < lifecycle.running);
    assert!(!chunks[lifecycle.compacting..lifecycle.running]
        .iter()
        .any(|c| state(c, SessionState::Idle)));
    // @step And the retry stream should begin processing on the committed context
    let resumed = live.inference_bodies().await;
    assert!(resumed.last().is_some_and(|body| body.contains(LABEL)));
    assert!(lifecycle.running < lifecycle.answer);
    // @step When the retry stream finishes and emits Done
    assert!(lifecycle.answer < lifecycle.done);
    // @step And the agent_loop reconciles the turn without a pending compaction candidate
    assert_eq!(committed_generation(&live), Some(1), "no second generation");
    // @step Then exactly one CompactionComplete should have been emitted for the turn
    assert_eq!(chunks.iter().filter(|c| is_completion(c)).count(), 1);
    // @step And the session status should transition to Idle
    assert!(lifecycle.idle < lifecycle.done);
    assert_eq!(live.session.get_status(), SessionStatus::Idle);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_pre_prompt_compaction_keeps_status_active_through_dag_construction() {
    // @step Given a user prompt would exceed the compaction threshold
    let mut live = Live::new(Trigger::PrePrompt).await;
    let chunks = live.turn().await;
    let lifecycle = Lifecycle::of(&chunks);
    // @step When stream_loop emits CompactionStarted before the API call
    let bodies = live.bodies().await;
    assert!(
        bodies
            .first()
            .is_some_and(|body| body.contains(&live.target())),
        "the builder runs before any inference request"
    );
    // @step Then the session status should be Compacting
    assert!(lifecycle.compacting < lifecycle.committed);
    // @step When the compaction builder durably commits the DAG
    assert_eq!(live.builders.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(committed_generation(&live), Some(1));
    // @step Then CompactionComplete should be emitted only after the durable commit
    assert!(!chunks[..lifecycle.committed].iter().any(is_completion));
    assert!(lifecycle.committed < lifecycle.completion);
    // @step When stream_loop emits CompactionContinuing
    // @step Then the agent-loop BackgroundOutput should set session status to Running
    assert!(lifecycle.completion < lifecycle.running);
    assert!(!chunks[lifecycle.compacting..lifecycle.running]
        .iter()
        .any(|c| state(c, SessionState::Idle)));
    // @step And the main stream should process the prompt on the committed context
    let inference = live.inference_bodies().await;
    assert_eq!(inference.len(), 1, "one main request after compaction");
    assert!(inference[0].contains(LABEL));
    assert!(!inference[0].contains("committed response"));
    // @step When the main stream finishes and emits Done
    assert!(lifecycle.answer < lifecycle.done);
    // @step And the agent_loop reconciles the turn without a pending compaction candidate
    assert_eq!(committed_generation(&live), Some(1), "no second generation");
    // @step Then exactly one CompactionComplete should have been emitted for the turn
    assert_eq!(chunks.iter().filter(|c| is_completion(c)).count(), 1);
    // @step And the session status should transition to Idle
    assert!(lifecycle.idle < lifecycle.done);
    assert_eq!(live.session.get_status(), SessionStatus::Idle);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_normal_response_without_compaction_does_not_emit_compaction_complete() {
    // @step Given an agent stream completes normally without triggering compaction
    let mut live = Live::new(Trigger::Normal).await;
    // @step When the stream emits Done
    let chunks = live.turn().await;
    let done = position(&chunks, 0, |c| matches!(c, StreamChunk::Done));
    // @step Then the session status should be Idle
    let idle = position(&chunks, 0, |c| state(c, SessionState::Idle));
    assert!(idle < done);
    assert_eq!(live.session.get_status(), SessionStatus::Idle);
    // @step And the agent_loop turn compaction application returns false
    assert_eq!(live.builders.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(committed_generation(&live), None);
    let mut inner = live.session.inner.lock().await;
    let unsubmitted =
        codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler(
            live.session.id,
            &inner,
            std::path::PathBuf::from(&live.session.project),
            live.session.clone(),
            None,
        );
    let applied = unsubmitted.apply(&mut inner, &live.session);
    codelet_tools::set_inject_summary_handler(live.session.id, None);
    codelet_tools::set_generate_compaction_handler(live.session.id, None);
    assert_eq!(applied, Ok(false));
    drop(inner);
    // @step And no CompactionComplete event should be emitted from the agent_loop
    assert!(!chunks.iter().any(is_completion));
    assert!(!chunks
        .iter()
        .any(|c| matches!(c, StreamChunk::CommittedCompaction { .. })));
    assert!(!live
        .session
        .get_buffered_output(500)
        .iter()
        .any(is_completion));
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_compaction_continuing_sets_status_to_running_instead_of_being_a_no_op() {
    // @step Given the session status is Compacting after CompactionStarted was emitted
    let (_directory, session, _input) = terminal::fixture();
    let output = BackgroundOutput::with_provider(session.clone(), "claude".into(), None);
    output.emit_compaction_started();
    assert_eq!(session.get_status(), SessionStatus::Compacting);
    let mut subscriber = session.subscribe_to_stream();
    // @step When the agent-loop BackgroundOutput receives a CompactionContinuing event
    output.emit(StreamEvent::CompactionContinuing);
    // @step Then the session status should be set to Running
    assert_eq!(session.get_status(), SessionStatus::Running);
    // @step And a SessionStateChange with Running should be emitted to stream subscribers
    let mut published = Vec::new();
    while let Ok(chunk) = subscriber.try_recv() {
        published.push(chunk);
    }
    assert!(
        published.iter().any(|c| state(c, SessionState::Running)),
        "{published:?}"
    );
}

/// Records every event stream_loop's compaction helper emits.
#[derive(Default)]
struct Recording(std::sync::Mutex<Vec<StreamEvent>>);

impl StreamOutput for Recording {
    fn emit(&self, event: StreamEvent) {
        self.0.lock().expect("recording lock").push(event);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_compaction_complete_is_not_emitted_prematurely_from_stream_loop() {
    let (_directory, session, _input) = terminal::fixture();
    let mut inner = session.inner.lock().await;
    let in_progress = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let tokens = std::sync::Arc::new(std::sync::Mutex::new(codelet_core::TokenState::default()));
    let output = Recording::default();
    // @step Given execute_compaction has completed the in-memory setup phase
    // @step When stream_loop processes the successful compaction result
    codelet_cli::interactive::execute_compaction_and_capture_events(
        &mut inner,
        in_progress.clone(),
        "continue the parser work",
        150_000,
        200_000,
        &tokens,
        &output,
    )
    .await
    .expect("in-memory compaction setup succeeds");
    assert!(in_progress.load(std::sync::atomic::Ordering::SeqCst));
    assert!(inner.has_compaction_working_view(), "builder view prepared");
    let events = output.0.lock().expect("recording lock");
    // @step Then stream_loop must emit CompactionContinuing instead of CompactionComplete
    let continuing = events
        .iter()
        .filter(|e| matches!(e, StreamEvent::CompactionContinuing))
        .count();
    assert_eq!(continuing, 1, "{events:?}");
    // @step And no CompactionComplete event should be emitted at this point
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, StreamEvent::CompactionComplete(_))),
        "{events:?}"
    );
    let durable = codelet_core::persistence::load_session(session.id).expect("durable manifest");
    assert!(durable.compaction_commit.is_none(), "nothing committed yet");
}
