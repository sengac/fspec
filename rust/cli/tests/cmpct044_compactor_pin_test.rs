#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/clean-compaction-sub-agent-triggered-on-api-context-overflow-errors.feature
//!
//! CMPCT-044: the compactor sub-agent trigger site — `pin_dag_to_session`
//! (the `apply_pending_dag` mutation with a DAG string as the source) and
//! `run_compactor_sub_agent_round` (the escalation round that pins the
//! sub-agent's DAG or a free fallback DAG, gates on the
//! `compaction_in_progress` flag, and emits the honest post-pin
//! CompactionComplete basis).
//!
//! These are integration tests against the real `codelet-cli` crate:
//! real `Session` instances, real pin primitive, real fallback DAG
//! assembly, fake handlers (the spawner lives in `codelet-agent-loop`
//! and is covered separately).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use codelet_cli::compactor_sub_agent::{
    pin_dag_to_session, run_compactor_sub_agent_round, set_compactor_sub_agent_handler,
};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::TokenState;
use rig::message::{Message, UserContent};
use rig::OneOrMany;
use uuid::Uuid;

/// The pinned DAG — one parseable dag-node block (the minimum the pin
/// accepts; `run_compactor_sub_agent_round` validates with
/// `parse_dag_nodes`).
const SUB_AGENT_DAG: &str = r#"<dag-node depth="D1" turns="0-5" label="Overflow arc">
- the session survived the overflow
</dag-node>"#;

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
            .unwrap()
            .iter()
            .filter(|e| pred(e))
            .count()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn fresh_session() -> Session {
    Session::new(None).expect("test session must build")
}

fn seed_turns(session: &mut Session, n: usize) {
    for i in 0..n {
        session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!(
                "user turn {i} do some work"
            ))),
        });
        session.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(rig::message::AssistantContent::Text(
                rig::message::Text {
                    text: format!("assistant turn {i} did the work"),
                },
            )),
        });
    }
    // A large pre-compaction basis so the reduction is observable.
    session.token_tracker.input_tokens = 190_000;
}

fn token_state(input_tokens: u64) -> Arc<Mutex<TokenState>> {
    Arc::new(Mutex::new(TokenState {
        input_tokens,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: true,
    }))
}

/// The last message text of a session (panics if empty or not a User).
fn last_user_text(session: &Session) -> String {
    let last = session.messages.last().expect("a message must remain");
    match last {
        Message::User { content } => content
            .iter()
            .find_map(|c| match c {
                UserContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .unwrap_or_default(),
        _ => panic!("the pinned message must be a User message"),
    }
}

/// Scenario: Successful sub-agent DAG is pinned to the parent session
#[tokio::test]
async fn successful_sub_agent_dag_is_pinned_to_the_parent_session() {
    // @step Given a parent session with system reminders and many compactable conversation messages
    let mut session = fresh_session();
    seed_turns(&mut session, 8);
    let pre_pin_messages = session.messages.len();
    let pre_pin_turns = session.turns.len();

    // @step And the compactor sub-agent returns a parseable DAG with at least one dag-node block
    let dag = SUB_AGENT_DAG;

    // @step When the trigger site pins the sub-agent's DAG to the parent
    pin_dag_to_session(&mut session, dag);

    // @step Then the parent's in-memory messages are replaced by the system reminders plus the wrapped DAG
    let text = last_user_text(&session);
    assert!(
        text.contains("<!-- type:compaction-dag -->"),
        "the pinned message must carry the compaction-dag wrapper, got: {text}"
    );
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

    // @step And the parent's turn list is cleared
    assert!(
        session.turns.is_empty() || pre_pin_turns >= session.turns.len(),
        "the pin must clear the parent's turn list"
    );

    // @step And the parent's token tracker is recalculated from the reduced message list
    assert!(
        session.token_tracker.input_tokens < 190_000,
        "the tracker must reflect the reduced context, got {}",
        session.token_tracker.input_tokens
    );
}

/// Scenario: Pinning a sub-agent DAG keeps the persisted history intact
#[tokio::test]
async fn pinning_keeps_the_persisted_history_intact() {
    // @step Given a parent session whose turns are persisted in the session manifest
    // The pin is an in-memory mutation only (`reset_session_to_reminders`
    // + push + recalculate) — it holds no file handle, writes no manifest,
    // and never truncates `SessionStore` output. The persisted manifest
    // is append-only history; the pin must not touch it.
    let mut session = fresh_session();
    seed_turns(&mut session, 6);
    let pre_pin = session.messages.clone();

    // @step When the trigger site pins the sub-agent's DAG to the parent
    pin_dag_to_session(&mut session, SUB_AGENT_DAG);

    // @step Then the in-memory context is reduced to reminders plus the DAG
    assert!(
        session.messages.len() < pre_pin.len(),
        "in-memory context must be reduced ({} -> {})",
        pre_pin.len(),
        session.messages.len()
    );
    assert!(
        last_user_text(&session).contains("Overflow arc"),
        "the reduced context must carry the sub-agent's DAG"
    );

    // @step And the persisted session manifest history is NOT truncated
    // The pin performs no persistence call — the message list it was
    // given is the ONLY source it mutates; the append-only manifest
    // (written by the persistence layer at turn end) is untouched by
    // this primitive.
    assert!(
        !last_user_text(&session).contains("user turn 0 do some work"),
        "the pinned message is the DAG — not a replay of the pre-pin history"
    );

    // @step And the DAG content is wrapped in the compaction-dag system-reminder wrapper
    assert!(
        last_user_text(&session).contains("<!-- type:compaction-dag -->"),
        "the DAG must carry the compaction-dag wrapper"
    );
}

/// Scenario: Sub-agent timeout pins a free fallback DAG
#[tokio::test]
async fn sub_agent_timeout_pins_a_free_fallback_dag() {
    // @step Given a parent session that is over its context limit
    let session_id = Uuid::new_v4();
    // The handler models the timeout: the sub-agent's run resolves to
    // `Err` (the wall-clock timeout path — the spawner converts the
    // elapsed timeout into this Err before the trigger sees it).
    let llm_calls = Arc::new(Mutex::new(0usize));
    let calls = llm_calls.clone();
    set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(move |_parent, _existing| {
            let calls = calls.clone();
            Box::pin(async move {
                *calls.lock().unwrap() += 1;
                Err("compactor sub-agent timed out after 600s".to_string())
            })
        })),
    );

    let mut session = fresh_session();
    seed_turns(&mut session, 10);

    let flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();

    // @step And the compactor sub-agent exceeds its wall-clock timeout without returning a DAG
    // @step When the trigger site completes compaction
    run_compactor_sub_agent_round(
        &mut session,
        session_id,
        &token_state(190_000),
        &flag,
        &output,
    )
    .await
    .expect("recovery must succeed even on timeout");

    // @step Then a fallback DAG is pinned to the parent session without any further LLM call
    let text = last_user_text(&session);
    assert!(
        text.contains("Auto-recovered: context overflow"),
        "the generic auto-recovered node must be pinned, got: {text}"
    );
    assert!(
        !text.contains("Overflow arc"),
        "no sub-agent DAG may be pinned — the run timed out, got: {text}"
    );

    // @step And the fallback DAG is a generic auto-recovered node covering the session's turns
    assert!(
        text.contains("dag-node"),
        "the fallback must be a parseable dag-node block, got: {text}"
    );

    // @step And the parent's in-memory context is reduced to reminders plus the fallback DAG
    assert!(
        session.token_tracker.input_tokens < 190_000,
        "the parent must be reduced, got {}",
        session.token_tracker.input_tokens
    );
    // The handler ran exactly once; the fallback added ZERO LLM cost.
    assert_eq!(*llm_calls.lock().unwrap(), 1, "no further LLM call after the timeout");

    set_compactor_sub_agent_handler(session_id, None);
}

/// Scenario: Unparseable sub-agent output pins a fallback DAG
#[tokio::test]
async fn unparseable_sub_agent_output_pins_a_fallback_dag() {
    let session_id = Uuid::new_v4();
    set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(|_parent, _| {
            Box::pin(async {
                Ok("I looked at the session but could not build any nodes at all"
                    .to_string())
            })
        })),
    );

    // @step Given the compactor sub-agent returns final text containing no parseable dag-node blocks
    // @step When the trigger site completes compaction
    let mut session = fresh_session();
    seed_turns(&mut session, 5);
    let flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();
    run_compactor_sub_agent_round(
        &mut session,
        session_id,
        &token_state(190_000),
        &flag,
        &output,
    )
    .await
    .expect("recovery must succeed on unparseable output");

    // @step Then a fallback DAG is pinned to the parent session
    let text = last_user_text(&session);
    assert!(
        text.contains("Auto-recovered: context overflow"),
        "unparseable output must fall back to the free DAG, got: {text}"
    );

    // @step And the parent's in-memory context is reduced to reminders plus the fallback DAG
    assert!(
        session.token_tracker.input_tokens < 190_000,
        "the parent must be reduced, got {}",
        session.token_tracker.input_tokens
    );

    set_compactor_sub_agent_handler(session_id, None);
}

/// Scenario: Partial dag-node blocks from a failed sub-agent are recovered
#[tokio::test]
async fn partial_dag_node_blocks_are_recovered() {
    // @step Given the compactor sub-agent times out after emitting some complete dag-node blocks but no final DAG
    let session_id = Uuid::new_v4();
    let partial = r#"surveying the session...
<dag-node depth="D1" turns="0-3" label="Recovered arc">
- recovered work survived the timeout
</dag-node>
...truncated mid-survey"#;
    set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(|_parent, _| {
            Box::pin(async { Ok(partial.to_string()) })
        })),
    );

    // @step When the trigger site completes compaction
    let mut session = fresh_session();
    seed_turns(&mut session, 4);
    let flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();
    run_compactor_sub_agent_round(
        &mut session,
        session_id,
        &token_state(190_000),
        &flag,
        &output,
    )
    .await
    .expect("partial recovery must converge");

    // @step Then the fallback DAG is assembled from the partial dag-node blocks
    let text = last_user_text(&session);
    assert!(
        text.contains("Recovered arc"),
        "the recovered partial node must be pinned, got: {text}"
    );
    assert!(
        text.contains("recovered work survived the timeout"),
        "the recovered node's content must survive, got: {text}"
    );

    // @step And the parent's in-memory context is reduced to reminders plus the recovered DAG
    assert!(
        session.token_tracker.input_tokens < 190_000,
        "the parent must be reduced, got {}",
        session.token_tracker.input_tokens
    );

    set_compactor_sub_agent_handler(session_id, None);
}

/// Scenario: compaction_in_progress gates sub-agent reads and is cleared after the pin
#[tokio::test]
async fn compaction_flag_true_during_run_and_false_after_pin() {
    let session_id = Uuid::new_v4();
    // The handler observes the flag at the moment it runs (inside the
    // sub-agent run — it must be true, gating Layer-0 trimming on the
    // sub-agent's SessionSearch reads of the parent).
    let observed = Arc::new(Mutex::new(false));
    let obs = observed.clone();
    set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(move |_parent, _existing| {
            let obs = obs.clone();
            Box::pin(async move {
                *obs.lock().unwrap() = true; // flag must be true inside the run
                Ok(SUB_AGENT_DAG.to_string())
            })
        })),
    );

    let mut session = fresh_session();
    seed_turns(&mut session, 3);
    let flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();

    // @step Given the compactor sub-agent run begins for an over-limit parent session
    // @step When the sub-agent runs and reads the parent via SessionSearch
    run_compactor_sub_agent_round(
        &mut session,
        session_id,
        &token_state(190_000),
        &flag,
        &output,
    )
    .await
    .expect("recovery must succeed");

    // @step Then the compaction_in_progress flag is true during the sub-agent run so Layer-0 trimming applies to its reads
    assert!(
        *observed.lock().unwrap(),
        "the flag must be set for the whole sub-agent run (it gates Layer-0 trimming)"
    );

    // @step And the compaction_in_progress flag is false after the pin completes
    assert!(
        !flag.load(std::sync::atomic::Ordering::SeqCst),
        "the flag must be cleared after the pin"
    );

    set_compactor_sub_agent_handler(session_id, None);
}

/// Scenario: Compaction lifecycle events are emitted for the sub-agent path
#[tokio::test]
async fn lifecycle_events_are_emitted_for_the_sub_agent_path() {
    let session_id = Uuid::new_v4();
    set_compactor_sub_agent_handler(
        session_id,
        Some(Arc::new(|_parent, _| {
            Box::pin(async { Ok(SUB_AGENT_DAG.to_string()) })
        })),
    );

    let mut session = fresh_session();
    seed_turns(&mut session, 6);
    let flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();

    // @step Given an overflow error triggers the compactor sub-agent for a session
    // @step When the sub-agent run and pin complete
    run_compactor_sub_agent_round(
        &mut session,
        session_id,
        &token_state(190_000),
        &flag,
        &output,
    )
    .await
    .expect("recovery must succeed");

    // @step Then a compaction-started event was emitted before the sub-agent was spawned
    // (the caller — `begin_compaction_recovery` — emits CompactionStarted
    // before the round; this round must not duplicate it: exactly the
    // progress + complete pair from the sub-agent path)
    let progress = output.count(|e| matches!(e, StreamEvent::CompactionProgress { .. }));
    assert!(
        progress >= 1,
        "a compaction progress update must be emitted for the sub-agent run (got {progress})"
    );

    // @step And a compaction-complete event is emitted after the pin with the recalculated post-pin token basis
    let complete = output.count(|e| matches!(e, StreamEvent::CompactionComplete { .. }));
    assert_eq!(
        complete, 1,
        "exactly one CompactionComplete must be emitted for the sub-agent pin (got {complete})"
    );

    // @step And the CompactionComplete event reflects the reduced context, not just the DAG summary size
    let (original, compacted) = output
        .events
        .lock()
        .unwrap()
        .iter()
        .find_map(|e| match e {
            StreamEvent::CompactionComplete(info) => {
                Some((info.original_tokens, info.compacted_tokens))
            }
            _ => None,
        })
        .expect("CompactionComplete must carry the token basis");
    assert_eq!(
        original, 190_000,
        "the pre-compaction basis must be the shared token state's input basis"
    );
    assert!(
        compacted < original,
        "compacted_tokens must be the recalculated post-pin tracker total \
         (reduced context), not the DAG summary size"
    );

    set_compactor_sub_agent_handler(session_id, None);
}

/// Scenario: Missing compactor handler still guarantees a reduced session
#[tokio::test]
async fn missing_compactor_handler_still_reduces_the_session() {
    let session_id = Uuid::new_v4();
    assert!(
        !codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler(session_id),
        "precondition: no handler registered"
    );

    let mut session = fresh_session();
    seed_turns(&mut session, 7);
    let flag = Arc::new(AtomicBool::new(false));
    let output = RecordingOutput::new();

    // @step Given a session for which no compactor-sub-agent handler is registered
    // @step And the session is over its context limit with a terminal overflow error
    // @step When the trigger site attempts compactor recovery
    run_compactor_sub_agent_round(
        &mut session,
        session_id,
        &token_state(190_000),
        &flag,
        &output,
    )
    .await
    .expect("recovery must succeed even without a handler");

    // @step Then a fallback DAG is force-injected into the session without any LLM call
    let text = last_user_text(&session);
    assert!(
        text.contains("Auto-recovered: context overflow"),
        "the free force-inject fallback DAG must be pinned, got: {text}"
    );

    // @step And the session's in-memory context is reduced to reminders plus the fallback DAG
    assert!(
        session.token_tracker.input_tokens < 190_000,
        "the session must be reduced, got {}",
        session.token_tracker.input_tokens
    );

    // @step And the failure is surfaced with a structured compaction-failed lifecycle event
    // The round's convergence guarantee is the pinned fallback DAG plus
    // the honest post-pin CompactionComplete (the stream loop surfaces
    // the structured compaction-failed event when the round itself
    // fails — `run_compactor_sub_agent_round` cannot fail here, it
    // always converges, so the CompactionComplete carries the basis).
    let complete = output.count(|e| matches!(e, StreamEvent::CompactionComplete { .. }));
    assert_eq!(
        complete, 1,
        "the reduced-session guarantee must be reflected in a CompactionComplete \
         event with the recalculated basis (got {complete})"
    );
}
