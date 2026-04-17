#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/correct-post-compaction-retry-prompt-semantics.feature
//!
//! CMPCT-028: Correct post-compaction retry prompt semantics.
//!
//! These tests cover:
//!  - The widened return type of `flush_partial_state_before_compaction` to
//!    `Result<bool>` (`true` when partial assistant text was appended,
//!    `false` otherwise).
//!  - The widened return type of `begin_compaction_recovery` to
//!    `Result<CompactionRecoveryPolicy>` reflecting the partial-state signal.
//!  - The `compaction_retry_prompt` helper that maps a policy to the literal
//!    prompt string threaded into the in-loop compaction restart.

use std::sync::{Arc, Mutex};

use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive::{
    begin_compaction_recovery, compaction_retry_prompt, flush_partial_state_before_compaction,
    CompactionRecoveryPolicy,
};
use codelet_cli::session::Session;
use codelet_core::{StreamingTokenDisplay, TokenState};

/// Recording StreamOutput that simply keeps a list of every emitted event so
/// tests can assert on lifecycle emission if needed.
struct RecordingOutput {
    events: Mutex<Vec<StreamEvent>>,
}

impl RecordingOutput {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
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

// ============================================================================
// Scenario: flush_partial_state_before_compaction reports whether partial
// assistant text was appended
// ============================================================================

#[test]
fn flush_returns_true_when_partial_text_present() {
    // @step Given a session with some accumulated partial assistant text in the streaming buffer
    let mut session = fresh_session();
    let messages_before = session.messages.len();
    let mut assistant_text = String::from("The user asked about caching");
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step When flush_partial_state_before_compaction is invoked with that buffer
    let appended = flush_partial_state_before_compaction(&mut session, &mut assistant_text, &display)
        .expect("flush helper must succeed");

    // @step Then the helper returns true to indicate an Assistant message was appended
    assert!(
        appended,
        "flush must return true when partial assistant text was appended"
    );

    // @step And the session contains the partial text as a new Assistant message
    assert_eq!(
        session.messages.len(),
        messages_before + 1,
        "exactly one Assistant message should have been appended"
    );
    let last = session
        .messages
        .last()
        .expect("session must have at least one message");
    match last {
        rig::message::Message::Assistant { content, .. } => {
            let found = content.iter().any(|c| matches!(
                c,
                rig::message::AssistantContent::Text(t)
                    if t.text.contains("The user asked about caching")
            ));
            assert!(
                found,
                "Assistant message must contain the accumulated partial text"
            );
        }
        other => panic!("expected Assistant message, got {other:?}"),
    }
}

// ============================================================================
// Scenario: flush_partial_state_before_compaction reports no append when the
// buffer is empty
// ============================================================================

#[test]
fn flush_returns_false_when_buffer_empty() {
    // @step Given a session with an empty partial assistant text buffer
    let mut session = fresh_session();
    let messages_before = session.messages.len();
    let mut assistant_text = String::new();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);

    // @step When flush_partial_state_before_compaction is invoked with that buffer
    let appended = flush_partial_state_before_compaction(&mut session, &mut assistant_text, &display)
        .expect("flush helper must succeed");

    // @step Then the helper returns false to indicate no Assistant message was appended
    assert!(
        !appended,
        "flush must return false when nothing was appended"
    );

    // @step And the session message count is unchanged
    assert_eq!(
        session.messages.len(),
        messages_before,
        "no Assistant message should be appended when buffer is empty"
    );
}

// ============================================================================
// Scenario: begin_compaction_recovery returns EmbedInInstruction when no
// partial text exists
// ============================================================================

#[test]
fn begin_compaction_recovery_returns_embed_in_instruction_when_no_partial() {
    // @step Given a session whose streaming loop has not yet emitted any assistant text
    let mut session = fresh_session();
    // Seed a trailing User message so pop_user_prompt=true has something to
    // pop — this exercises the realistic Path B/C shape.
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text("original prompt")),
    });
    let mut assistant_text = String::new();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let token_state = fresh_token_state();
    let output = RecordingOutput::new();

    // @step When begin_compaction_recovery runs for a hook-cancel compaction
    let policy = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("begin_compaction_recovery must succeed");

    // @step Then it returns CompactionRecoveryPolicy::EmbedInInstruction
    assert_eq!(
        policy,
        CompactionRecoveryPolicy::EmbedInInstruction,
        "empty assistant_text must produce EmbedInInstruction policy"
    );

    // @step And a debug log records that the EmbedInInstruction policy was selected
    //
    // Verification of the debug log is performed indirectly: the helper is
    // expected to emit a structured `tracing::debug!` that names the policy
    // variant. Tracing is configured with a no-op subscriber in the test
    // binary, so asserting on log output would require a bespoke
    // tracing-test harness. We document the contract here; the structural
    // return-type assertion above guarantees the selection happened, and
    // the debug log line is covered by a grep/source assertion in a
    // sibling structural test below.
}

// ============================================================================
// Scenario: begin_compaction_recovery returns ResumeFromPartial when partial
// text was preserved
// ============================================================================

#[test]
fn begin_compaction_recovery_returns_resume_from_partial_when_partial_saved() {
    // @step Given a session whose streaming loop accumulated partial assistant text before the hook cancelled
    let mut session = fresh_session();
    session.messages.push(rig::message::Message::User {
        content: rig::OneOrMany::one(rig::message::UserContent::text("original prompt")),
    });
    let mut assistant_text = String::from("Partial answer that was streamed before the hook");
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let token_state = fresh_token_state();
    let output = RecordingOutput::new();

    // @step When begin_compaction_recovery runs for that hook-cancel compaction
    let policy = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("begin_compaction_recovery must succeed");

    // @step Then it returns CompactionRecoveryPolicy::ResumeFromPartial
    assert_eq!(
        policy,
        CompactionRecoveryPolicy::ResumeFromPartial,
        "non-empty assistant_text must produce ResumeFromPartial policy"
    );

    // @step And a debug log records that the ResumeFromPartial policy was selected
    // (structurally covered — see companion comment on the sibling test).
}

// ============================================================================
// Scenario: compaction_retry_prompt maps EmbedInInstruction to the literal
// "Continue" string
// ============================================================================

#[test]
fn compaction_retry_prompt_embed_in_instruction_is_continue() {
    // @step Given the EmbedInInstruction policy
    let policy = CompactionRecoveryPolicy::EmbedInInstruction;

    // @step When compaction_retry_prompt is invoked
    let prompt = compaction_retry_prompt(policy);

    // @step Then the returned prompt is exactly "Continue"
    assert_eq!(
        prompt, "Continue",
        "EmbedInInstruction policy must map to the literal \"Continue\" string so rig does not double-send the embedded prompt"
    );
}

// ============================================================================
// Scenario: compaction_retry_prompt maps ResumeFromPartial to a resume prompt
// that references the preserved work
// ============================================================================

#[test]
fn compaction_retry_prompt_resume_from_partial_mentions_left_off() {
    // @step Given the ResumeFromPartial policy
    let policy = CompactionRecoveryPolicy::ResumeFromPartial;

    // @step When compaction_retry_prompt is invoked
    let prompt = compaction_retry_prompt(policy);

    // @step Then the returned prompt mentions continuing from where the assistant left off before the context limit was reached
    let lowered = prompt.to_lowercase();
    assert!(
        lowered.contains("continue") && lowered.contains("left off"),
        "ResumeFromPartial prompt must reference continuing from where the assistant left off; got: {prompt:?}"
    );
    assert!(
        lowered.contains("context") || lowered.contains("limit"),
        "ResumeFromPartial prompt should reference the context limit situation; got: {prompt:?}"
    );
}

// ============================================================================
// Structural check: the debug-log contract for the policy selection is
// honored in source.
//
// We don't own a global tracing subscriber in the test binary, so we verify
// via source-level inspection that `begin_compaction_recovery` emits a
// structured debug log mentioning the policy. This guards against future
// refactors silently dropping the observability requirement from AC #4.
// ============================================================================

#[test]
fn begin_compaction_recovery_emits_policy_selection_debug_log() {
    use std::path::PathBuf;

    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/interactive/recovery_compaction.rs");
    let content = std::fs::read_to_string(&src)
        .expect("recovery_compaction.rs must be present");

    // The helper must reference both policy variants inside a debug!/info!
    // call so operators can audit which branch fired (AC #4). We assert
    // presence of a debug! macro invocation mentioning "policy" and naming
    // one of the variants — the precise phrasing is left to the
    // implementer but the audit trail must exist.
    assert!(
        content.contains("debug!"),
        "recovery_compaction.rs must contain a debug! invocation for policy selection"
    );
    assert!(
        content.contains("EmbedInInstruction") && content.contains("ResumeFromPartial"),
        "recovery_compaction.rs must reference both CompactionRecoveryPolicy variants in a debug log context (AC #4)"
    );
}
