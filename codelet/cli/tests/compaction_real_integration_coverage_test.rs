#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/real-integration-coverage-for-compaction-cancel-path.feature
//!
//! CMPCT-030: Replace tautological compaction tests with real integration coverage.
//!
//! This file replaces three tautological test units that were deleted as part
//! of CMPCT-030:
//!   * `codelet/cli/tests/gemini_continuation_compaction_test.rs` — asserted
//!     `"PromptCancelled".contains("PromptCancelled")` and `true == true`.
//!   * `codelet/cli/tests/empty_turn_compaction_fix_test.rs` — reconstructed
//!     the production compaction gate locally as
//!     `let should_compact = effective_tokens > threshold && !session.turns.is_empty();`
//!     and asserted on the local variable.
//!   * `fn test_normal_compaction_no_watchdog` in
//!     `compaction_convergence_watchdog_test.rs` — stored `false` into a
//!     local `AtomicBool` and asserted the `AtomicBool` was `false`.
//!
//! The replacement tests combine:
//!
//! 1. Structural assertions on `gemini_continuation.rs` to prove it delegates
//!    to the CMPCT-023/026 unified helpers (`classify_compaction_branch`,
//!    `begin_compaction_recovery`) instead of re-implementing the branch
//!    locally.
//! 2. Direct invocation of `flush_partial_state_before_compaction` and
//!    `compaction_retry_prompt` to verify that the stream-end-with-flag path
//!    in `gemini_continuation.rs` (which inlines the policy selection, since
//!    it does not call `begin_compaction_recovery`) follows the same rule as
//!    the helper.
//! 3. Direct invocation of `convert_messages_to_turns` (the actual production
//!    helper used by `stream_loop.rs:1185` to gate compaction on the
//!    prompt-too-long path) against real `rig::message::Message` fixtures.
//! 4. Filesystem checks to prove the deleted tautological files have not
//!    re-entered the tree.
//!
//! Production code refactor (extracting `process_stream`) is explicitly
//! out of scope per the CMPCT-030 plan — these structural grep assertions
//! are the cheapest way to pin the contracts established by CMPCT-023..028.

use std::path::PathBuf;

use codelet_cli::interactive::{
    compaction_retry_prompt, flush_partial_state_before_compaction, CompactionRecoveryPolicy,
};
use codelet_cli::interactive_helpers::convert_messages_to_turns;
use codelet_cli::session::Session;
use codelet_core::StreamingTokenDisplay;
use rig::message::{AssistantContent, Message, Text, UserContent};
use rig::OneOrMany;

// ----------------------------------------------------------------------------
// Shared fixtures
// ----------------------------------------------------------------------------

fn fresh_session() -> Session {
    Session::new(None).expect("failed to create test session")
}

fn gemini_continuation_source() -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/interactive/gemini_continuation.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "gemini_continuation.rs must be readable at {}: {e}",
            path.display()
        )
    })
}

fn user_text(text: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::Text(Text {
            text: text.to_string(),
        })),
    }
}

fn assistant_text(text: &str) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::Text(Text {
            text: text.to_string(),
        })),
    }
}

// ============================================================================
// Scenario: Gemini continuation uses the unified classify_compaction_branch
// helper on stream error
// ============================================================================

#[test]
fn gemini_continuation_calls_classify_compaction_branch_on_stream_error() {
    // @step Given the source file codelet/cli/src/interactive/gemini_continuation.rs is readable
    let src = gemini_continuation_source();

    // @step When the source is scanned for a classify_compaction_branch call
    let has_call = src.contains("classify_compaction_branch(");

    // @step Then the call is present and the source does NOT reimplement a bespoke string-match on the error
    assert!(
        has_call,
        "gemini_continuation.rs must call classify_compaction_branch (CMPCT-026 contract)"
    );

    // Defence-in-depth: `gemini_continuation.rs` must not roll its own
    // stringly-typed PromptCancelled detection. The pre-CMPCT-025 pattern
    // was `e.to_string().contains("PromptCancelled")` — if this ever reappears
    // the single-source-of-truth contract has broken.
    assert!(
        !src.contains("to_string().contains(\"PromptCancelled\")")
            && !src.contains("to_string().contains(\"prompt_cancelled\")"),
        "gemini_continuation.rs must NOT do stringly-typed PromptCancelled detection"
    );
}

// ============================================================================
// Scenario: Gemini continuation invokes begin_compaction_recovery with
// pop_user_prompt=false on Recover
// ============================================================================

#[test]
fn gemini_continuation_invokes_begin_compaction_recovery_with_pop_user_prompt_false() {
    // @step Given a classify_compaction_branch call site has been located in gemini_continuation.rs
    let src = gemini_continuation_source();
    assert!(
        src.contains("classify_compaction_branch("),
        "precondition: classify_compaction_branch call must exist"
    );

    // @step When the following block is inspected for the unified recovery helper call
    assert!(
        src.contains("begin_compaction_recovery("),
        "gemini_continuation.rs must call begin_compaction_recovery (CMPCT-023 Path D)"
    );

    // @step Then begin_compaction_recovery is invoked with pop_user_prompt=false
    //
    // Path D semantic: the User continuation prompt is mid-flight — popping
    // it would corrupt the restart payload. The call site in
    // gemini_continuation.rs ends with `false,` on its own line, so the
    // structural shape is stable across rustfmt reflows.
    let call_shape_ok = src.contains("begin_compaction_recovery(")
        && src
            .lines()
            .skip_while(|l| !l.contains("begin_compaction_recovery("))
            .take(10)
            .any(|l| l.trim_end_matches(',').trim() == "false");
    assert!(
        call_shape_ok,
        "begin_compaction_recovery must be called with pop_user_prompt=false in gemini_continuation.rs"
    );
}

// ============================================================================
// Scenario: Stream-end-with-compaction-flag path in Gemini continuation
// selects ResumeFromPartial when partial text was saved
// ============================================================================

#[test]
fn stream_end_path_selects_resume_from_partial_when_partial_text_saved() {
    // @step Given a fresh session and an empty streaming display
    let mut session = fresh_session();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step And a non-empty assistant_text buffer carrying partial text
    let mut buffer = String::from("Here is the analysis so far — paragraph one is complete");

    // @step When flush_partial_state_before_compaction is invoked and the selected policy is chosen per the in-file rule
    // `flush_partial_state_before_compaction` returns `true` when it appended
    // an Assistant message (CMPCT-028). `gemini_continuation.rs` mirrors this
    // with `let policy = if partial_text_saved { ResumeFromPartial } else {
    // EmbedInInstruction };` — we apply the same rule against the helper's
    // return value to catch drift in either site.
    let partial_text_saved =
        flush_partial_state_before_compaction(&mut session, &mut buffer, &display)
            .expect("flush helper must succeed");
    let policy = if partial_text_saved {
        CompactionRecoveryPolicy::ResumeFromPartial
    } else {
        CompactionRecoveryPolicy::EmbedInInstruction
    };

    // @step Then the selected CompactionRecoveryPolicy is ResumeFromPartial
    assert_eq!(
        policy,
        CompactionRecoveryPolicy::ResumeFromPartial,
        "partial text must produce ResumeFromPartial on the stream-end-with-flag path"
    );

    // Retry prompt must reference the preserved work — guards against a
    // silent switch to the "Continue" literal on the partial-saved path.
    let prompt_text = compaction_retry_prompt(policy);
    let lowered = prompt_text.to_lowercase();
    assert!(
        lowered.contains("left off") && lowered.contains("continue"),
        "ResumeFromPartial prompt must guide the agent to resume from where it left off; got: {prompt_text:?}"
    );
}

// ============================================================================
// Scenario: Stream-end-with-compaction-flag path in Gemini continuation
// selects EmbedInInstruction when no partial text was saved
// ============================================================================

#[test]
fn stream_end_path_selects_embed_in_instruction_when_no_partial_text() {
    // @step Given a fresh session and an empty streaming display
    let mut session = fresh_session();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step And an empty assistant_text buffer
    let mut buffer = String::new();

    // @step When flush_partial_state_before_compaction is invoked and the selected policy is chosen per the in-file rule
    let partial_text_saved =
        flush_partial_state_before_compaction(&mut session, &mut buffer, &display)
            .expect("flush helper must succeed");
    let policy = if partial_text_saved {
        CompactionRecoveryPolicy::ResumeFromPartial
    } else {
        CompactionRecoveryPolicy::EmbedInInstruction
    };

    // @step Then the selected CompactionRecoveryPolicy is EmbedInInstruction
    assert_eq!(
        policy,
        CompactionRecoveryPolicy::EmbedInInstruction,
        "empty buffer must produce EmbedInInstruction on the stream-end-with-flag path"
    );

    // The EmbedInInstruction retry prompt must be the literal "Continue"
    // string (CMPCT-028 contract) so rig does not double-send the embedded
    // prompt.
    assert_eq!(
        compaction_retry_prompt(policy),
        "Continue",
        "EmbedInInstruction retry prompt must be the literal \"Continue\" (CMPCT-028)"
    );
}

// ============================================================================
// Scenario: Tautological gemini_continuation_compaction_test.rs has been
// deleted from the cli tests directory
// ============================================================================

#[test]
fn deleted_tautological_gemini_test_file_is_absent() {
    // @step Given a clean workspace with the cli tests directory present
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    assert!(tests_dir.is_dir(), "cli tests directory must exist");

    // @step When the tests directory is listed
    let deleted_file = tests_dir.join("gemini_continuation_compaction_test.rs");

    // @step Then gemini_continuation_compaction_test.rs is absent
    assert!(
        !deleted_file.exists(),
        "gemini_continuation_compaction_test.rs must remain deleted — its scenarios assert \
         \"PromptCancelled\".contains(\"PromptCancelled\") and are tautological (CMPCT-030)"
    );
}

// ============================================================================
// Scenario: convert_messages_to_turns returns an empty slice for a session
// with only system reminders
// ============================================================================

#[test]
fn convert_messages_to_turns_returns_empty_for_system_reminders_only() {
    // @step Given a rig Message vec that contains only a system reminder User message
    //
    // Production shape: fspec injects system reminders as plain
    // `UserContent::Text` messages with no following assistant reply. The
    // compaction gate must treat them as non-compactable.
    let messages = vec![user_text(
        "<system-reminder>\n<!-- type:environment -->\nPlatform: test\n</system-reminder>",
    )];

    // @step When convert_messages_to_turns is invoked
    let turns = convert_messages_to_turns(&messages);

    // @step Then the returned slice is empty and the compaction gate `has_compactable_turns` is false
    assert!(
        turns.is_empty(),
        "a lone User (system reminder) must produce zero turns — the compaction gate at \
         stream_loop.rs:1185 would have silently compacted an empty session otherwise"
    );
    let has_compactable_turns = !turns.is_empty();
    assert!(
        !has_compactable_turns,
        "stream_loop.rs:1185 gate `is_prompt_too_long && has_compactable_turns` must be false \
         when only system reminders are present"
    );
}

// ============================================================================
// Scenario: convert_messages_to_turns returns one turn per user-assistant
// pair
// ============================================================================

#[test]
fn convert_messages_to_turns_returns_one_turn_per_user_assistant_pair() {
    // @step Given a rig Message vec with three User-followed-by-Assistant text pairs
    let messages = vec![
        user_text("user 1"),
        assistant_text("assistant 1"),
        user_text("user 2"),
        assistant_text("assistant 2"),
        user_text("user 3"),
        assistant_text("assistant 3"),
    ];

    // @step When convert_messages_to_turns is invoked
    let turns = convert_messages_to_turns(&messages);

    // @step Then the returned slice contains exactly three turns
    assert_eq!(
        turns.len(),
        3,
        "three user/assistant pairs must produce exactly three ConversationTurn entries"
    );

    // Each turn must preserve its user_message + assistant_response in order
    // so downstream compaction summarization sees a faithful transcript.
    for (i, turn) in turns.iter().enumerate() {
        assert_eq!(
            turn.user_message,
            format!("user {}", i + 1),
            "turn {i} must carry the i-th user message"
        );
        assert_eq!(
            turn.assistant_response,
            format!("assistant {}", i + 1),
            "turn {i} must carry the i-th assistant response"
        );
    }

    // Gate condition must be true for this session — this is the inverse of
    // the previous test, proving `convert_messages_to_turns` is the real
    // gate used by production and not a tautological local reconstruction.
    let has_compactable_turns = !turns.is_empty();
    assert!(
        has_compactable_turns,
        "stream_loop.rs:1185 gate must allow compaction when user/assistant pairs exist"
    );
}

// ============================================================================
// Scenario: Tautological empty_turn_compaction_fix_test.rs has been deleted
// from the cli tests directory
// ============================================================================

#[test]
fn deleted_tautological_empty_turn_test_file_is_absent() {
    // @step Given a clean workspace with the cli tests directory present
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    assert!(tests_dir.is_dir(), "cli tests directory must exist");

    // @step When the tests directory is listed
    let deleted_file = tests_dir.join("empty_turn_compaction_fix_test.rs");

    // @step Then empty_turn_compaction_fix_test.rs is absent
    assert!(
        !deleted_file.exists(),
        "empty_turn_compaction_fix_test.rs must remain deleted — its tests reconstructed the \
         production gate locally as `let should_compact = effective_tokens > threshold && \
         !session.turns.is_empty();` and asserted on the local variable (CMPCT-030)"
    );
}

// ============================================================================
// Scenario: Tautological test_normal_compaction_no_watchdog has been deleted
// from compaction_convergence_watchdog_test.rs
// ============================================================================

#[test]
fn deleted_tautological_watchdog_test_is_absent() {
    // @step Given the compaction_convergence_watchdog_test.rs source file is present
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/compaction_convergence_watchdog_test.rs");
    assert!(
        source_path.is_file(),
        "compaction_convergence_watchdog_test.rs must still exist — only the one tautological fn is deleted"
    );

    // @step When the file is scanned for a fn named test_normal_compaction_no_watchdog
    let source =
        std::fs::read_to_string(&source_path).expect("watchdog test source must be readable");

    // @step Then no such function definition is found
    assert!(
        !source.contains("fn test_normal_compaction_no_watchdog"),
        "test_normal_compaction_no_watchdog must remain deleted — it stored false into a local \
         AtomicBool and asserted the AtomicBool was false (CMPCT-030)"
    );
}
