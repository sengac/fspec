#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/preserve-partial-assistant-text-and-token-tracker-on-hook-triggered-compaction.feature
//!
//! CMPCT-024: Preserve partial assistant text and token tracker on hook-triggered compaction.
//!
//! These tests cover the extracted `flush_partial_state_before_compaction` helper
//! that is invoked from the stream loop's compaction-cancel branch. The helper is
//! the smallest unit that captures the state-preservation responsibilities
//! (BUG 2 + BUG 6) without needing a real rig stream.

use codelet_cli::interactive::flush_partial_state_before_compaction;
use codelet_cli::session::Session;
use codelet_core::StreamingTokenDisplay;

fn fresh_session() -> Session {
    // Some test environments may not have providers configured — skip if so.
    Session::new(None).expect("failed to create test session")
}

/// Scenario: Partial assistant text is saved into the conversation before compaction
#[test]
fn partial_assistant_text_is_saved_before_compaction() {
    let mut session = fresh_session();

    // @step Given a session whose last message is a user prompt
    let user_message_count_before = session.messages.len();
    // (session starts with whatever reminders are injected; we just measure delta)

    // @step And the streaming loop has accumulated some assistant text in its buffer
    let mut assistant_text = String::from("Here is my analy");
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step When the compaction hook cancels the current stream
    flush_partial_state_before_compaction(&mut session, &mut assistant_text, &display)
        .expect("flush helper must succeed");

    // @step Then the accumulated assistant text is appended to the session as an Assistant message
    assert_eq!(
        session.messages.len(),
        user_message_count_before + 1,
        "exactly one Assistant message should have been appended"
    );
    let last = session
        .messages
        .last()
        .expect("must have at least one message");
    match last {
        rig::message::Message::Assistant { content, .. } => {
            let has_expected_text = content.iter().any(|c| match c {
                rig::message::AssistantContent::Text(t) => t.text == "Here is my analy",
                _ => false,
            });
            assert!(
                has_expected_text,
                "Assistant message should contain the accumulated partial text"
            );
        }
        other => panic!("expected Assistant message, got {other:?}"),
    }

    // @step And the assistant text buffer is cleared
    assert!(
        assistant_text.is_empty(),
        "assistant_text buffer should be cleared after flushing"
    );
}

/// Scenario: Token tracker is flushed with the latest streaming display values
#[test]
fn token_tracker_is_flushed_with_streaming_display_values() {
    let mut session = fresh_session();

    // @step Given a session with a fresh token tracker
    session.token_tracker.cumulative_billed_input = 0;
    session.token_tracker.cumulative_billed_output = 0;

    // @step And the streaming display shows a non-zero input token count
    // @step And the streaming display shows a non-zero output token count
    // We seed the display via the `new` constructor with prev_* values and then
    // apply a synthetic Usage chunk so both values are populated.
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

    let mut empty_text = String::new();

    // @step When the compaction hook cancels the current stream
    flush_partial_state_before_compaction(&mut session, &mut empty_text, &display)
        .expect("flush helper must succeed");

    // @step Then the session token tracker reflects the streaming display input tokens
    assert_eq!(
        session.token_tracker.input_tokens, 1_200,
        "token_tracker.input_tokens must match display input"
    );

    // @step And the session token tracker reflects the streaming display output tokens
    assert_eq!(
        session.token_tracker.output_tokens, 340,
        "token_tracker.output_tokens must match display output"
    );

    // @step And cumulative billed output accumulation reflects the per-turn delta (TOKEN-001)
    //
    // TOKEN-001 fixed the pre-existing bug where every call site into
    // `update_from_usage` passed `output_tokens: 0` to `ApiTokenUsage::new`,
    // dropping every per-turn output from the cumulative billing accumulator.
    // The helper now computes the per-turn delta via
    // `TokenTracker::compute_output_delta` before calling `update_from_usage`,
    // so a fresh tracker observing cumulative output of 340 must end up with
    // `cumulative_billed_output == 340`.
    assert_eq!(
        session.token_tracker.cumulative_billed_output, 340,
        "TOKEN-001: the first flush from a fresh tracker must accumulate the \
         full per-turn delta (340) into cumulative_billed_output"
    );
}

/// Scenario: Empty partial text does not pollute the conversation
#[test]
fn empty_partial_text_does_not_append_assistant_message() {
    let mut session = fresh_session();

    // @step Given a session whose last message is a user prompt
    let messages_before = session.messages.len();

    // @step And the streaming loop buffer contains no assistant text
    let mut assistant_text = String::new();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step When the compaction hook cancels the current stream
    flush_partial_state_before_compaction(&mut session, &mut assistant_text, &display)
        .expect("flush helper must succeed");

    // @step Then the number of messages in the session is unchanged
    assert_eq!(
        session.messages.len(),
        messages_before,
        "no Assistant message should be appended when text is empty"
    );

    // @step And no Assistant message with empty content is appended
    // (No new message — the last message (if any) is not one we just appended.
    // This assertion is covered by the length check above.)
}
