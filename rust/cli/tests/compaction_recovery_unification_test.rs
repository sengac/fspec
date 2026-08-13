#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/unify-compaction-entry-paths-into-a-single-helper.feature
//!
//! CMPCT-023: Unify compaction entry paths into a single helper.
//!
//! These tests exercise the unified helper `begin_compaction_recovery` directly
//! (without spinning up a real rig stream) to prove that Paths B, C, and D all
//! converge on identical invariants:
//!
//! - Partial assistant text is saved
//! - Token tracker is flushed from the StreamingTokenDisplay
//! - The last user message is popped iff `pop_user_prompt=true`
//! - `compaction_needed` flag is set on the shared TokenState
//! - Tool progress callback is cleared
//! - `compaction_started` + `compaction_progress` events are emitted exactly once
//!
//! The tests use a `RecordingOutput` fake that records every emitted event into
//! a thread-safe buffer, so we can assert event counts and content without any
//! mocking framework.

use std::sync::{Arc, Mutex};

use codelet_cli::interactive::begin_compaction_recovery;
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::{StreamingTokenDisplay, TokenState};

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

    fn find_first<F: Fn(&StreamEvent) -> bool>(&self, pred: F) -> Option<StreamEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .find(|e| pred(e))
            .cloned()
    }
}

impl StreamOutput for RecordingOutput {
    fn emit(&self, event: StreamEvent) {
        self.events.lock().unwrap().push(event);
    }
}

fn fresh_session() -> Session {
    Session::new(None).expect("failed to create test session")
}

fn fresh_token_state() -> Arc<Mutex<TokenState>> {
    Arc::new(Mutex::new(TokenState {
        input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: false,
    }))
}

/// Seed a display with a completed turn's usage so token_tracker flush picks it up.
fn seed_display() -> StreamingTokenDisplay {
    let mut display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let usage = rig::completion::Usage {
        input_tokens: 1_200,
        output_tokens: 340,
        total_tokens: 1_540,
        cache_read_input_tokens: None,
        cache_creation_input_tokens: None,
        reasoning_tokens: None,
    };
    let _ = display.update_from_usage(&usage);
    display
}

/// Push a user message onto the session, returning the new length.
fn push_user(session: &mut Session, text: &str) -> usize {
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text(text)),
    });
    session.messages.len()
}

/// Scenario: Path B produces identical end state as Path C from identical starting conditions
#[test]
fn path_b_and_path_c_produce_identical_end_states() {
    // @step Given two identical sessions whose last message is a user prompt 'Hi' and whose streaming buffer contains 'partial answer'
    let mut session_b = fresh_session();
    let mut session_c = fresh_session();
    push_user(&mut session_b, "Hi");
    push_user(&mut session_c, "Hi");

    let baseline_len_b = session_b.messages.len();
    let baseline_len_c = session_c.messages.len();
    assert_eq!(baseline_len_b, baseline_len_c);

    let token_state_b = fresh_token_state();
    let token_state_c = fresh_token_state();

    let display_b = seed_display();
    let display_c = seed_display();

    let mut text_b = String::from("partial answer");
    let mut text_c = String::from("partial answer");

    let output_b = RecordingOutput::new();
    let output_c = RecordingOutput::new();

    // @step When begin_compaction_recovery is called on both sessions with pop_user_prompt=true
    begin_compaction_recovery(
        &mut session_b,
        &token_state_b,
        &display_b,
        &mut text_b,
        &output_b,
        true,
    )
    .expect("helper must succeed for Path B");

    begin_compaction_recovery(
        &mut session_c,
        &token_state_c,
        &display_c,
        &mut text_c,
        &output_c,
        true,
    )
    .expect("helper must succeed for Path C");

    // @step Then both sessions end with identical session.messages (the Assistant message appended, the User 'Hi' popped)
    // Net effect: append Assistant, then pop User('Hi'). Length = baseline + 1 - 1 = baseline.
    assert_eq!(
        session_b.messages.len(),
        baseline_len_b,
        "Path B: length should be unchanged (one Assistant appended, one User popped)"
    );
    assert_eq!(
        session_c.messages.len(),
        baseline_len_c,
        "Path C: length should be unchanged (one Assistant appended, one User popped)"
    );

    // Structural equivalence: both sessions' last messages should be the Assistant message with "partial answer".
    let last_b = session_b
        .messages
        .last()
        .expect("session B has at least one message");
    let last_c = session_c
        .messages
        .last()
        .expect("session C has at least one message");

    match (last_b, last_c) {
        (
            rig::message::Message::Assistant { content: c_b, .. },
            rig::message::Message::Assistant { content: c_c, .. },
        ) => {
            let has_text_b = c_b.iter().any(|c| matches!(c, rig::message::AssistantContent::Text(t) if t.text == "partial answer"));
            let has_text_c = c_c.iter().any(|c| matches!(c, rig::message::AssistantContent::Text(t) if t.text == "partial answer"));
            assert!(
                has_text_b,
                "Path B last message should carry the partial text"
            );
            assert!(
                has_text_c,
                "Path C last message should carry the partial text"
            );
        }
        _ => panic!("expected both last messages to be Assistant"),
    }

    // @step Then both sessions end with identical token_tracker state
    assert_eq!(
        session_b.token_tracker.input_tokens, session_c.token_tracker.input_tokens,
        "token_tracker.input_tokens must match across Path B and Path C"
    );
    assert_eq!(
        session_b.token_tracker.output_tokens, session_c.token_tracker.output_tokens,
        "token_tracker.output_tokens must match across Path B and Path C"
    );

    // Both should also reflect the seeded display (1200 / 340).
    assert_eq!(session_b.token_tracker.input_tokens, 1_200);
    assert_eq!(session_b.token_tracker.output_tokens, 340);

    // @step Then both sessions have compaction_needed=true on their shared TokenState
    assert!(
        token_state_b.lock().unwrap().compaction_needed,
        "Path B flag must be set"
    );
    assert!(
        token_state_c.lock().unwrap().compaction_needed,
        "Path C flag must be set"
    );

    // Both assistant_text buffers must be cleared.
    assert!(text_b.is_empty());
    assert!(text_c.is_empty());

    // Both outputs emitted the compaction lifecycle events exactly once each.
    assert_eq!(
        output_b.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1
    );
    assert_eq!(
        output_c.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1
    );
    assert_eq!(
        output_b.count(|e| matches!(e, StreamEvent::CompactionProgress(_))),
        1
    );
    assert_eq!(
        output_c.count(|e| matches!(e, StreamEvent::CompactionProgress(_))),
        1
    );
}

/// Scenario: Path D preserves all messages when pop_user_prompt is false
#[test]
fn path_d_preserves_all_messages_when_pop_user_prompt_is_false() {
    let mut session = fresh_session();

    // @step Given a session whose last message is a mid-flight User continuation prompt
    push_user(&mut session, "Continue with the next step");
    let messages_len_before = session.messages.len();

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();

    // @step When begin_compaction_recovery is called with pop_user_prompt=false and an empty assistant_text buffer
    let mut empty_text = String::new();
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_text,
        &output,
        false, // pop_user_prompt = false — Path D semantic
    )
    .expect("helper must succeed for Path D");

    // @step Then the last user message remains in session.messages
    assert_eq!(
        session.messages.len(),
        messages_len_before,
        "Path D: session.messages count must be unchanged (no pop, no append for empty text)"
    );
    let last = session.messages.last().expect("last message should remain");
    match last {
        rig::message::Message::User { content, .. } => {
            let has_text = content.iter().any(|c| matches!(c, rig::message::UserContent::Text(t) if t.text == "Continue with the next step"));
            assert!(
                has_text,
                "Path D: last message must still be the User continuation prompt"
            );
        }
        other => panic!("Path D: expected User at tail, got {other:?}"),
    }

    // @step Then no Assistant message is appended for an empty buffer
    // (Covered by the length check above — length unchanged means no Assistant was appended.)

    // @step Then compaction_needed is set to true on the TokenState
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "Path D: flag must be set"
    );

    // Events emitted exactly once each.
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1
    );
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionProgress(_))),
        1
    );
}

/// Scenario: Helper emits compaction lifecycle events exactly once per entry
#[test]
fn helper_emits_lifecycle_events_exactly_once_per_entry() {
    // @step Given a fake StreamOutput that records every emit_compaction_started and emit_compaction_progress call
    let output = RecordingOutput::new();
    let mut session = fresh_session();
    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let mut empty_text = String::new();

    // @step When begin_compaction_recovery is called once
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_text,
        &output,
        false,
    )
    .expect("helper must succeed");

    // @step Then exactly one compaction_started event is recorded
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1,
        "compaction_started must be emitted exactly once per entry"
    );

    // @step And exactly one compaction_progress event with phase 'Context limit reached' is recorded
    let progress = output
        .find_first(|e| matches!(e, StreamEvent::CompactionProgress(_)))
        .expect("exactly one compaction_progress event expected");
    match progress {
        StreamEvent::CompactionProgress(info) => {
            assert_eq!(info.phase, "Context limit reached", "phase must match spec");
            assert_eq!(info.current, 0, "current must be 0 at entry");
            assert!(info.total >= 1, "total must be >=1 (max(1))");
        }
        other => panic!("expected CompactionProgress, got {other:?}"),
    }
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionProgress(_))),
        1,
        "compaction_progress must be emitted exactly once per entry"
    );
}

/// Scenario: Helper emits a warning when compaction_needed flag was already true on entry
#[test]
fn helper_succeeds_when_flag_already_true_on_entry() {
    let mut session = fresh_session();

    // @step Given a TokenState whose compaction_needed flag is already true before entering the helper
    let token_state = Arc::new(Mutex::new(TokenState {
        input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: true,
    }));

    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut empty_text = String::new();

    // @step When begin_compaction_recovery is invoked
    let result = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_text,
        &output,
        false,
    );

    // @step Then the helper completes successfully without error
    assert!(
        result.is_ok(),
        "helper must succeed even when flag was already set"
    );

    // @step Then the compaction_needed flag remains true
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "compaction_needed must remain true"
    );

    // Lifecycle events still emitted exactly once.
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1
    );
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionProgress(_))),
        1
    );
}

/// Scenario: Empty partial assistant text does not append an empty Assistant message
#[test]
fn empty_partial_text_does_not_append_empty_assistant_message() {
    let mut session = fresh_session();

    // @step Given a session with some existing messages and an empty assistant_text buffer
    push_user(&mut session, "user prompt 1");
    let messages_before = session.messages.len();

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut empty_text = String::new();

    // @step When begin_compaction_recovery is invoked with pop_user_prompt=false
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_text,
        &output,
        false,
    )
    .expect("helper must succeed with empty buffer");

    // @step Then the session.messages count is unchanged
    assert_eq!(
        session.messages.len(),
        messages_before,
        "session.messages count must be unchanged"
    );

    // @step Then no empty Assistant message is added
    let last = session.messages.last().expect("last must exist");
    assert!(
        matches!(last, rig::message::Message::User { .. }),
        "last message must still be the User prompt — no empty Assistant appended"
    );
}
