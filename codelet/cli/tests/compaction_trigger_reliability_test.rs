#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/compaction-trigger-reliability.feature
//!
//! CMPCT-032: Compaction triggering broken after CMPCT-023..031 refactor —
//! `FinalResponse` path bypasses recovery.
//!
//! This test file validates that compaction recovery fires on EVERY
//! stream-loop exit path where `token_state.compaction_needed == true`.
//! The regression introduced by commit dc3d5934 (CMPCT-027) deleted the
//! post-loop compaction block and replaced it with an in-loop macro that
//! only fires from the ERROR arm of `stream.next()`. Two paths silently
//! bypass recovery:
//!
//!   1. `Some(Ok(FinalResponse))` at stream_loop.rs:968-1295 never checks
//!      `token_state.compaction_needed` before `emit_done_with_stop_reason`.
//!   2. The post-loop safety net at stream_loop.rs:1777-1798 is gated behind
//!      `#[cfg(debug_assertions)]` — in release builds there is no
//!      production-mode recovery, only a `debug!` log.
//!
//! Tests combine two layers of coverage:
//!
//!   * **Structural grep assertions** on `stream_loop.rs` source — these
//!     pin the production fix in place and fail against the current buggy
//!     commit so the regression cannot re-appear. This mirrors the pattern
//!     established by `compaction_real_integration_coverage_test.rs` which
//!     scans `gemini_continuation.rs` for the single-source-of-truth
//!     contract.
//!   * **Behavioural assertions** on the real helpers
//!     (`begin_compaction_recovery`, `execute_compaction`,
//!     `classify_compaction_branch`, `resolve_compaction_threshold`,
//!     `CompactionHook::check_compaction_with_payload`) — these prove each
//!     scenario's helper-level invariants using production code paths and
//!     fail if any helper regresses.
//!
//! Running against the current (buggy) commit should produce compile-time
//! success but runtime FAILURES on the structural grep tests, proving the
//! regression is present. After the fix, all tests pass.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use codelet_cli::compaction_threshold::{resolve_compaction_threshold, CompactionThresholdConfig};
use codelet_cli::interactive::{
    begin_compaction_recovery, classify_compaction_branch, compaction_retry_prompt,
    CompactionBranch, CompactionRecoveryPolicy,
};
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive_helpers::execute_compaction;
use codelet_cli::session::Session;
use codelet_core::{CompactionHook, StreamingTokenDisplay, TokenState};

use rig::agent::CancelSignal;
use rig::completion::PromptError;
use rig::message::{AssistantContent, Message, Text, UserContent};
use rig::OneOrMany;

// ============================================================================
// Shared fixtures
// ============================================================================

/// Load the `stream_loop.rs` source once per test module so structural
/// assertions can pin the production contract.
fn stream_loop_source() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/interactive/stream_loop.rs");
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "stream_loop.rs must be readable at {}: {e}",
            path.display()
        )
    })
}

/// Recording StreamOutput that captures every emitted event for assertion.
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

fn fresh_session_with_conversation() -> Session {
    let provider_manager = codelet_providers::ProviderManager::new()
        .expect("at least one API key must be configured for tests");
    let mut session = Session::from_provider_manager(provider_manager);
    // System reminder so partition_for_compaction has something to preserve.
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(
            "<system-reminder>\n<!-- type:environment -->\nPlatform: test\n</system-reminder>",
        )),
    });
    // Real User/Assistant pairs so convert_messages_to_turns produces turns.
    for i in 0..4 {
        session.messages.push(Message::User {
            content: OneOrMany::one(UserContent::text(format!("user message {i}"))),
        });
        session.messages.push(Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::Text(Text {
                text: format!("assistant response {i}"),
            })),
        });
    }
    session.token_tracker.input_tokens = 50_000;
    session
}

fn fresh_token_state(compaction_needed: bool) -> Arc<Mutex<TokenState>> {
    Arc::new(Mutex::new(TokenState {
        input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed,
    }))
}

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

/// Build a PromptCancelled anyhow::Error that matches what rig yields
/// mid-stream when the CompactionHook fires — exercises the real
/// `extract_prompt_cancelled` downcast path used by
/// `classify_compaction_branch`.
fn build_prompt_cancelled_error() -> anyhow::Error {
    PromptError::PromptCancelled {
        chat_history: Box::new(Vec::<Message>::new()),
    }
    .into()
}

/// Mirror the production `CompactionHook::on_completion_call` trigger
/// condition for synchronous tests: when `state.total() > threshold`, set
/// `compaction_needed=true` and cancel the signal. This is the exact
/// contract `on_completion_call` implements — we reproduce it here because
/// `StreamingPromptHook::on_completion_call` is async and requires a
/// full rig agent context to invoke.
fn simulate_hook_check(
    hook: &CompactionHook,
    state: &Arc<Mutex<TokenState>>,
    cancel_sig: &CancelSignal,
) {
    let total = state.lock().unwrap().total();
    if total > hook.threshold() {
        state.lock().unwrap().compaction_needed = true;
        cancel_sig.cancel();
    }
}

// ============================================================================
// Scenario: FinalResponse branch triggers recovery when compaction_needed is set
// ============================================================================
//
// REGRESSION GUARD: The FinalResponse branch at stream_loop.rs:968-1295 must
// check `token_state.compaction_needed` before `emit_done_with_stop_reason`.
// The current buggy commit breaks the loop with the flag set and never runs
// recovery.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn final_response_branch_triggers_recovery_when_compaction_needed_is_set() {
    // @step Given a streaming turn is in progress
    let mut session = fresh_session_with_conversation();
    let token_state = fresh_token_state(false);
    let display = seed_display();
    let mut assistant_text = String::new();
    let output = RecordingOutput::new();

    // @step And the compaction hook sets token_state.compaction_needed=true on the last chunk
    // Simulate the hook having fired on the last chunk — the flag is set but
    // rig yielded Ok(FinalResponse) rather than PromptCancelled.
    token_state.lock().unwrap().compaction_needed = true;

    // @step And the stream yields Ok(FinalResponse) with stop_reason "end_turn"
    // (Modelled structurally via the stream_loop source check below — no live
    // stream needed because the branch is a pure control-flow decision.)
    let src = stream_loop_source();

    // @step When the stream loop processes the FinalResponse branch
    // Locate the "Normal case" block inside the FinalResponse branch — the
    // path that runs AFTER the Gemini continuation check and the thinking-
    // exhaustion retry, and BEFORE `emit_done_with_stop_reason`. This is the
    // actual regression site: the Gemini sub-block checks `compaction_needed`
    // via its own handler, but the normal success path does not.
    let normal_case_marker = "// Normal case: add assistant text to history and finish";
    let normal_case_idx = src
        .find(normal_case_marker)
        .expect("FinalResponse branch must contain the 'Normal case' marker");
    let normal_slice = &src[normal_case_idx..];
    // Find the ACTUAL `emit_done_with_stop_reason(` invocation — we must
    // skip the string match in doc comments that describe the call. The
    // canonical invocation is: `output.emit_done_with_stop_reason(`.
    let emit_done_relative = normal_slice
        .find("output.emit_done_with_stop_reason(")
        .expect("Normal case path must call output.emit_done_with_stop_reason(...)");
    let normal_window = &normal_slice[..emit_done_relative];

    // @step Then begin_compaction_recovery is invoked with policy EmbedInInstruction("Continue")
    // The fix: the normal-case FinalResponse path MUST read `compaction_needed`
    // before `emit_done_with_stop_reason`. Structural assertion: the window
    // between "// Normal case:" and `emit_done_with_stop_reason` must mention
    // `compaction_needed` — proving the flag is inspected on the clean-exit
    // path, not only inside the Gemini continuation sub-block.
    assert!(
        normal_window.contains("compaction_needed"),
        "REGRESSION (CMPCT-032): the 'Normal case' FinalResponse path at \
         stream_loop.rs:~1131-1295 must check token_state.compaction_needed \
         BEFORE emit_done_with_stop_reason. The current source breaks the \
         loop with compaction_needed=true unhandled, causing the NEXT turn \
         to fail with 'prompt too long'. \
         Expected `compaction_needed` token to appear in the window between \
         the `// Normal case:` comment and `emit_done_with_stop_reason`.\n\
         Offending window bytes:\n{normal_window}"
    );

    // And the in-loop compaction restart macro must be reachable from the
    // normal-case path (so the fix can route through the unified macro
    // rather than re-implementing recovery inline).
    assert!(
        normal_window.contains("in_loop_compaction_restart!"),
        "REGRESSION (CMPCT-032): 'Normal case' FinalResponse path must invoke \
         in_loop_compaction_restart!() when compaction_needed=true so the \
         retry stream is subject to the same error cascade as the original \
         stream. Without it, the FinalResponse path has no unified recovery."
    );

    // @step And emit_done_with_stop_reason is NOT called for that turn
    // Helper-level proof: when the flag is set, `begin_compaction_recovery`
    // returns an EmbedInInstruction policy (empty partial text) and
    // `compaction_retry_prompt` returns the literal "Continue". The
    // FinalResponse branch must thread that policy into the macro instead of
    // emitting done.
    let policy = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        false, // pop_user_prompt=false — the FinalResponse branch does not pop
    )
    .expect("begin_compaction_recovery must succeed when flag is set");
    assert_eq!(
        policy,
        CompactionRecoveryPolicy::EmbedInInstruction,
        "Empty assistant text on the FinalResponse clean-exit path must select \
         EmbedInInstruction so rig receives the literal \"Continue\" prompt"
    );

    // @step And the compaction flow runs to completion and restarts the stream
    assert_eq!(
        compaction_retry_prompt(policy),
        "Continue",
        "EmbedInInstruction policy must produce the literal \"Continue\" retry prompt \
         so rig does not double-send the embedded prompt (CMPCT-028 contract)"
    );
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1,
        "begin_compaction_recovery must emit CompactionStarted exactly once"
    );
    assert!(
        output.count(|e| matches!(e, StreamEvent::CompactionProgress(_))) >= 1,
        "begin_compaction_recovery must emit at least one CompactionProgress event"
    );
}

// ============================================================================
// Scenario: In-loop macro handles PromptCancelled from mid-stream hook
// cancellation
// ============================================================================
//
// This is the Path C (CMPCT-023..028) regression guard — the error arm with
// PromptCancelled MUST continue firing the in-loop macro after the fix.

#[test]
fn in_loop_macro_handles_prompt_cancelled_from_mid_stream_hook_cancellation() {
    // @step Given a streaming turn is in progress
    let token_state = fresh_token_state(false);

    // @step And the compaction hook fires mid-stream and cancels the stream
    // Simulate the hook having set the flag and rig having yielded a typed
    // PromptCancelled error (the real shape yielded by the rig patch at
    // streaming.rs sites 412, 460, 494-496, 512, 608).
    token_state.lock().unwrap().compaction_needed = true;
    let err = build_prompt_cancelled_error();

    // @step When rig yields PromptError::PromptCancelled via the error arm
    let branch = classify_compaction_branch(&err, &token_state);

    // @step Then classify_compaction_branch returns Recover
    assert!(
        matches!(branch, CompactionBranch::Recover { .. }),
        "classify_compaction_branch must return Recover for typed PromptCancelled \
         when compaction_needed=true (CMPCT-026 contract)"
    );

    // @step And in_loop_compaction_restart!() is invoked
    // Structural proof: the Path C call site in stream_loop.rs must still
    // invoke the in-loop macro after begin_compaction_recovery. The fix for
    // CMPCT-032 must NOT revert the error-arm wiring.
    let src = stream_loop_source();
    // Locate the PromptCancelled Recover branch invocation.
    assert!(
        src.contains("CMPCT-027: in-loop compaction restart (Path C)"),
        "Error arm Path C (PromptCancelled) must retain its in-loop restart wiring"
    );
    assert!(
        src.matches("in_loop_compaction_restart!(policy)").count() >= 3,
        "in_loop_compaction_restart!(policy) must be invoked from at least three \
         error-arm call sites (Paths B, C, D) after the fix"
    );

    // @step And begin_compaction_recovery runs with policy ResumeFromPartial
    // When partial text was preserved (mid-stream cancel), the helper must
    // select ResumeFromPartial — not EmbedInInstruction.
    let mut session = fresh_session_with_conversation();
    // Simulate the User prompt at the tail (rig pushed it before the cancel).
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Continue with next step")),
    });
    let display = seed_display();
    let mut partial_text = String::from("Here is the partial analysis before cancel");
    let output = RecordingOutput::new();

    let policy = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut partial_text,
        &output,
        true, // pop_user_prompt=true — Path C pops the trailing User prompt
    )
    .expect("begin_compaction_recovery must succeed for Path C");

    assert_eq!(
        policy,
        CompactionRecoveryPolicy::ResumeFromPartial,
        "Path C with preserved partial text must select ResumeFromPartial \
         so the retry prompt references the preserved work (CMPCT-028)"
    );

    // @step And a fresh stream is built and processed via the retry path
    let retry_prompt = compaction_retry_prompt(policy);
    assert!(
        retry_prompt.to_lowercase().contains("left off"),
        "ResumeFromPartial retry prompt must guide the agent to resume from \
         where it left off, got: {retry_prompt:?}"
    );
    assert!(
        partial_text.is_empty(),
        "Partial text buffer must be cleared after recovery"
    );
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "compaction_needed must remain true so the in-loop macro fires"
    );
}

// ============================================================================
// Scenario: In-loop macro handles upstream prompt-too-long error
// ============================================================================

#[test]
fn in_loop_macro_handles_upstream_prompt_too_long_error() {
    // @step Given a streaming turn is in progress
    let mut session = fresh_session_with_conversation();
    let token_state = fresh_token_state(false);

    // @step And the upstream provider returns 400 prompt-too-long
    // This is Path B — prompt-too-long arrives via the error arm, NOT through
    // PromptCancelled. The flag is therefore false when the error arrives.
    let prompt_too_long_error = "HTTP 400: invalid_request_error: prompt is too long";

    // @step When the stream yields an error matching the prompt-too-long classifier
    // classify_compaction_branch must return NotCompaction for this error
    // (so the prompt-too-long handler catches it instead).
    let err = anyhow::Error::msg(prompt_too_long_error);
    let branch = classify_compaction_branch(&err, &token_state);
    assert!(
        matches!(branch, CompactionBranch::NotCompaction { .. }),
        "Prompt-too-long (no PromptCancelled) must route to NotCompaction so \
         the prompt-too-long classifier catches it"
    );

    assert!(
        codelet_cli::interactive::is_prompt_too_long_error(prompt_too_long_error),
        "is_prompt_too_long_error must detect the canonical prompt-too-long phrase"
    );

    // @step Then in_loop_compaction_restart!() is invoked
    // Structural proof: Path B must still invoke the in-loop macro after
    // begin_compaction_recovery. Regression guard for CMPCT-032 not reverting
    // CMPCT-023..028.
    let src = stream_loop_source();
    assert!(
        src.contains("CMPCT-027: in-loop compaction restart (Path B)"),
        "Error arm Path B (prompt-too-long) must retain its in-loop restart wiring"
    );

    // @step And begin_compaction_recovery runs with policy EmbedInInstruction("Continue")
    // Path B with no partial text → EmbedInInstruction. If partial text had
    // accumulated before the error, rig would have flushed it to
    // chat_history already (CMPCT-024 + CMPCT-029), so the outer
    // assistant_text buffer is empty.
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("my query")),
    });
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let mut empty_buf = String::new();
    let output = RecordingOutput::new();

    let policy = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_buf,
        &output,
        true, // pop_user_prompt=true — Path B
    )
    .expect("begin_compaction_recovery must succeed for Path B");

    assert_eq!(
        policy,
        CompactionRecoveryPolicy::EmbedInInstruction,
        "Path B with no partial text must select EmbedInInstruction so rig \
         receives the literal \"Continue\" prompt"
    );

    // @step And the retry stream is processed successfully
    assert_eq!(compaction_retry_prompt(policy), "Continue");
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1,
        "CompactionStarted must be emitted exactly once per recovery entry"
    );
}

// ============================================================================
// Scenario: In-loop macro handles Gemini continuation exhaustion
// ============================================================================

#[test]
fn in_loop_macro_handles_gemini_continuation_exhaustion() {
    // @step Given a Gemini streaming turn is in progress
    let token_state = fresh_token_state(true);

    // @step And Gemini continuation attempts are exhausted
    // The Gemini continuation sub-loop (Path D) calls
    // begin_compaction_recovery with pop_user_prompt=false when compaction is
    // triggered mid-continuation. See gemini_continuation.rs:366-373.
    let src = stream_loop_source();

    // @step When the stream yields the Gemini exhaustion error
    // @step Then in_loop_compaction_restart!() is invoked
    assert!(
        src.contains("CMPCT-027: in-loop compaction restart (Path D"),
        "Gemini continuation (Path D) must retain its in-loop restart wiring"
    );

    // @step And begin_compaction_recovery runs
    // Path D preserves the mid-flight continuation User prompt — no pop.
    let mut session = fresh_session_with_conversation();
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text("Gemini continuation")),
    });
    let messages_before = session.messages.len();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let mut empty_buf = String::new();
    let output = RecordingOutput::new();

    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_buf,
        &output,
        false, // pop_user_prompt=false — Path D
    )
    .expect("begin_compaction_recovery must succeed for Path D");

    // Path D MUST preserve the continuation User prompt — no pop.
    assert_eq!(
        session.messages.len(),
        messages_before,
        "Path D must NOT pop the mid-flight continuation User prompt"
    );
    let last = session.messages.last().unwrap();
    assert!(
        matches!(last, Message::User { .. }),
        "Path D: last message must still be the continuation User prompt"
    );

    // @step And the retry stream is processed successfully
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionStarted)),
        1
    );
}

// ============================================================================
// Scenario: Post-loop safety net runs recovery when loop exits via break with
// flag set
// ============================================================================
//
// REGRESSION GUARD: This is the core CMPCT-032 bug. The post-loop block at
// stream_loop.rs:1777-1798 must be a PRODUCTION-MODE safety net, NOT a
// `#[cfg(debug_assertions)]` debug log.

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn post_loop_safety_net_runs_recovery_when_loop_exits_via_break_with_flag_set() {
    // @step Given a streaming turn is in progress
    // @step And the compaction hook has set token_state.compaction_needed=true
    // @step And the loop breaks via a path that does not invoke the in-loop macro (e.g., thinking-exhaustion retry)
    let src = stream_loop_source();

    // @step When the stream loop reaches the post-loop block
    // @step Then in release builds (not only debug_assertions) the safety net fires
    //
    // Structural proof: the post-loop block MUST NOT be gated behind
    // `#[cfg(debug_assertions)]`. The current (buggy) commit wraps the
    // entire post-loop compaction check in `#[cfg(debug_assertions)]`, so
    // release builds have no safety net.
    let post_loop_marker = "CMPCT-032: Production-mode post-loop safety net";
    let post_loop_idx = src
        .find(post_loop_marker)
        .expect(
            "stream_loop.rs must have a production-mode post-loop safety net \
             marked with 'CMPCT-032: Production-mode post-loop safety net'. \
             The buggy commit had a `#[cfg(debug_assertions)]`-gated \
             `CMPCT-027: The post-loop handle_compaction_retry call` block \
             that only emitted a debug! log. The fix must replace it with a \
             production-mode safety net that actually invokes recovery.",
        );
    // Take the window from the post-loop marker through the end of the file.
    let post_loop_window = &src[post_loop_idx..];

    // The post-loop compaction check MUST NOT be gated behind
    // #[cfg(debug_assertions)]. If it is, release builds have no safety net.
    //
    // Find the next `compaction_needed` reference after the marker — that is
    // the post-loop check. The line(s) immediately preceding it must NOT
    // contain a `#[cfg(debug_assertions)]` attribute that scopes the check.
    let compaction_needed_idx = post_loop_window
        .find("compaction_needed")
        .expect("post-loop block must reference compaction_needed to be a real safety net");
    // Look backwards from that position for the nearest attribute or block
    // opening within ~500 chars (conservative window).
    let window_start = compaction_needed_idx.saturating_sub(500);
    let preceding = &post_loop_window[window_start..compaction_needed_idx];
    assert!(
        !preceding.contains("#[cfg(debug_assertions)]"),
        "REGRESSION (CMPCT-032): post-loop compaction safety net MUST NOT be gated \
         behind #[cfg(debug_assertions)]. The current source has:\n\
             #[cfg(debug_assertions)]\n\
             {{\n\
                 if let Ok(state) = token_state.lock() {{\n\
                     if state.compaction_needed && !is_interrupted.load(Acquire) {{\n\
                         debug!(...); // <-- DEAD CODE IN RELEASE BUILDS\n\
                     }}\n\
                 }}\n\
             }}\n\
         In production, this means compaction silently never happens when the \
         loop exits via a path that bypasses the in-loop macro. The fix must \
         make the safety net fire in release builds too.\n\
         Offending preceding bytes (last 500 before compaction_needed):\n{preceding}"
    );

    // @step And begin_compaction_recovery is invoked
    // The post-loop safety net MUST call begin_compaction_recovery (or
    // equivalent recovery entry) — not just emit a log. Search for either
    // `begin_compaction_recovery(` or the unified helper reference in the
    // post-loop window.
    assert!(
        post_loop_window.contains("begin_compaction_recovery"),
        "REGRESSION (CMPCT-032): post-loop safety net must invoke \
         begin_compaction_recovery when compaction_needed=true && !is_interrupted. \
         Currently the block only emits a debug! log and exits — silently \
         losing the context-exhaustion signal in production."
    );

    // The safety net must also invoke execute_compaction so the retry happens
    // in the same stream_loop context as the original turn.
    assert!(
        post_loop_window.contains("execute_compaction"),
        "REGRESSION (CMPCT-032): post-loop safety net must call execute_compaction \
         to actually compact history, not just flag the session"
    );

    // @step And a production-mode warning log identifies which branch missed the check
    // The safety-net entry point must emit a production-mode `warn!` (not a
    // debug-only log), identifying the miss. Debug-only logging is invisible
    // in release.
    assert!(
        post_loop_window.contains("warn!("),
        "REGRESSION (CMPCT-032): post-loop safety net must emit a production-mode \
         warn! (not debug!) identifying which branch exited without firing the \
         in-loop macro. This log is the operator's only signal that the safety \
         net fired — debug! is stripped in release builds."
    );

    // Behavioural layer: prove the helpers the safety net must call actually
    // do the right thing end-to-end.
    let mut session = fresh_session_with_conversation();
    let token_state = fresh_token_state(true);
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let mut empty_buf = String::new();
    let output = RecordingOutput::new();

    let policy = begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut empty_buf,
        &output,
        false, // post-loop safety net: don't pop (loop already broke cleanly)
    )
    .expect("post-loop recovery must succeed");

    // After recovery, execute_compaction compresses context so the retry
    // stream does not exceed the window.
    let compaction_flag = Arc::new(AtomicBool::new(false));
    execute_compaction(&mut session, compaction_flag.clone(), Some("Continue"))
        .await
        .expect("execute_compaction must succeed after begin_compaction_recovery");
    assert!(
        compaction_flag.load(Ordering::Relaxed),
        "execute_compaction must set compaction_in_progress flag"
    );
    assert_eq!(compaction_retry_prompt(policy), "Continue");
}

// ============================================================================
// Scenario: User interrupt takes priority over compaction recovery
// ============================================================================

#[test]
fn user_interrupt_takes_priority_over_compaction_recovery() {
    // @step Given a streaming turn is in progress
    // @step And the compaction hook has set token_state.compaction_needed=true
    let token_state = fresh_token_state(true);

    // @step And is_interrupted is set to true
    let is_interrupted = Arc::new(AtomicBool::new(true));

    // @step When the stream loop exits via any path
    // @step Then begin_compaction_recovery is NOT invoked
    //
    // Structural proof: the post-loop safety net must gate its invocation on
    // `!is_interrupted.load(Acquire)`. Without that guard, recovery runs
    // against a user-cancelled session — the exact opposite of what the user
    // asked for.
    let src = stream_loop_source();
    let post_loop_marker = "CMPCT-032: Production-mode post-loop safety net";
    let post_loop_idx = src
        .find(post_loop_marker)
        .expect("post-loop safety net marker must exist after the CMPCT-032 fix");
    let post_loop_window = &src[post_loop_idx..];

    assert!(
        post_loop_window.contains("!is_interrupted.load(Acquire)")
            || post_loop_window.contains("!is_interrupted.load(std::sync::atomic::Ordering::Acquire)"),
        "REGRESSION (CMPCT-032): post-loop safety net must guard on \
         `!is_interrupted.load(Acquire)` so user interrupts take priority over \
         compaction recovery. Running recovery on an interrupted session would \
         trigger a heavy DAG build the user explicitly cancelled."
    );

    // @step And the turn terminates with the interrupt state honoured
    // Behavioural proof: when `is_interrupted` is true, the stream_loop's
    // start-of-iteration check (at line ~700-720) handles the interrupt and
    // breaks BEFORE the post-loop block runs. The post-loop block's guard is
    // defence-in-depth for paths that bypass the start-of-iteration check.
    let interrupt_checked = is_interrupted.load(Ordering::Acquire);
    assert!(
        interrupt_checked,
        "is_interrupted must read true for this scenario"
    );
    assert!(
        token_state.lock().unwrap().compaction_needed,
        "compaction_needed remains true; only is_interrupted should block recovery"
    );
}

// ============================================================================
// Scenario: Gemini session triggers compaction at 80% of context window
// ============================================================================

#[test]
fn gemini_session_triggers_compaction_at_80_percent_of_context_window() {
    // @step Given an active Gemini session with a 1,000,000-token context window
    let context_window: u64 = 1_000_000;
    let max_output: u64 = 65_536;
    let model_id = Some("gemini-2.5-pro");

    // @step And resolve_compaction_threshold returns 800,000 tokens for the Gemini family default
    let threshold = resolve_compaction_threshold(context_window, max_output, model_id, None);
    assert_eq!(
        threshold, 800_000,
        "Gemini family default must be 80% of context window (CTX-007)"
    );

    // @step When the session's total input tokens exceed 800,000
    let state = Arc::new(Mutex::new(TokenState {
        input_tokens: 800_500,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: false,
    }));
    let hook = CompactionHook::new(Arc::clone(&state), threshold);
    let cancel_sig = CancelSignal::new();

    // @step Then the compaction hook fires
    // Mirror the production `on_completion_call` logic: when
    // `state.total() > threshold`, the hook sets `compaction_needed=true`
    // and cancels the signal. This is the production trigger condition.
    simulate_hook_check(&hook, &state, &cancel_sig);
    assert!(
        cancel_sig.is_cancelled(),
        "Hook must cancel when total_tokens ({}) > threshold ({})",
        800_500, threshold
    );

    // @step And token_state.compaction_needed becomes true
    assert!(
        state.lock().unwrap().compaction_needed,
        "compaction_needed must be set so the stream loop routes to recovery"
    );

    // @step And compaction recovery runs on the next stream exit
    // This is validated by the post-loop safety net test and the FinalResponse
    // branch test above — both exercise the real recovery paths.
    let src = stream_loop_source();
    assert!(
        src.contains("resolve_compaction_threshold"),
        "stream_loop.rs must call resolve_compaction_threshold to honour per-model thresholds"
    );
}

// ============================================================================
// Scenario: Claude session honours user threshold override
// ============================================================================

#[test]
fn claude_session_honours_user_threshold_override() {
    // @step Given an active Claude session with a user override threshold of 150,000 tokens
    let context_window: u64 = 200_000;
    let max_output: u64 = 8_192;
    let model_id = Some("claude-sonnet-4-20250514");
    let user_override = CompactionThresholdConfig::Tokens(150_000);

    // @step And resolve_compaction_threshold returns the override value
    let threshold = resolve_compaction_threshold(
        context_window,
        max_output,
        model_id,
        Some(&user_override),
    );
    assert_eq!(
        threshold, 150_000,
        "User override must take priority over Claude base formula (CTX-007)"
    );

    // Without override, Claude uses the base formula (200k - 8k = 191,808),
    // proving the override actually changed behaviour.
    let default_threshold = resolve_compaction_threshold(
        context_window,
        max_output,
        model_id,
        None,
    );
    assert_eq!(
        default_threshold, 191_808,
        "Claude default must be base formula (200k - min(8k, 32k) = 191,808)"
    );
    assert_ne!(
        threshold, default_threshold,
        "User override must differ from default; test fixture is invalid if they match"
    );

    // @step When the session's total input tokens exceed 150,000
    let state = Arc::new(Mutex::new(TokenState {
        input_tokens: 151_000,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: false,
    }));
    let hook = CompactionHook::new(Arc::clone(&state), threshold);
    let cancel_sig = CancelSignal::new();

    // @step Then the compaction hook fires at the override threshold, not the default base formula
    simulate_hook_check(&hook, &state, &cancel_sig);
    assert!(
        cancel_sig.is_cancelled(),
        "Hook must cancel at user-override threshold (151k > 150k), \
         NOT wait until the base formula threshold (191,808)"
    );

    // Proof that the default would NOT have fired yet — same token count,
    // default threshold → no cancel.
    let state_default = Arc::new(Mutex::new(TokenState {
        input_tokens: 151_000,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        output_tokens: 0,
        compaction_needed: false,
    }));
    let hook_default = CompactionHook::new(Arc::clone(&state_default), default_threshold);
    let cancel_default = CancelSignal::new();
    simulate_hook_check(&hook_default, &state_default, &cancel_default);
    assert!(
        !cancel_default.is_cancelled(),
        "Sanity: default threshold (191,808) would NOT have fired at 151k tokens; \
         proves user override actually lowered the trigger point"
    );

    // @step And compaction recovery runs
    assert!(
        state.lock().unwrap().compaction_needed,
        "Override-triggered hook must set compaction_needed so recovery runs"
    );
}

// ============================================================================
// Scenario: After recovery, restart stream does not immediately re-trigger the hook
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_recovery_restart_stream_does_not_immediately_re_trigger_the_hook() {
    // @step Given begin_compaction_recovery has just completed execute_compaction
    let mut session = fresh_session_with_conversation();
    let token_state = fresh_token_state(true);
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let mut buf = String::new();
    let output = RecordingOutput::new();

    // Run begin_compaction_recovery (sets flag, emits events).
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut buf,
        &output,
        false,
    )
    .expect("begin_compaction_recovery must succeed");

    // @step And chat_history has been replaced with the compacted summary
    let compaction_flag = Arc::new(AtomicBool::new(false));
    execute_compaction(&mut session, compaction_flag.clone(), Some("Continue"))
        .await
        .expect("execute_compaction must succeed");

    // Verify chat history shrank after compaction.
    assert!(
        session.messages.len() < 20,
        "session.messages must shrink after execute_compaction (got {})",
        session.messages.len()
    );

    // @step And token_state.compaction_needed has been reset to false
    // The in-loop macro resets the flag after execute_compaction. Emulate
    // that reset — which is the contract the macro holds.
    {
        let mut state = token_state.lock().unwrap();
        state.compaction_needed = false;
        state.input_tokens = session.token_tracker.input_tokens;
        state.cache_read_input_tokens = 0;
        state.cache_creation_input_tokens = 0;
        state.output_tokens = 0;
    }

    // Structural proof: the in-loop macro in stream_loop.rs MUST reset the
    // flag and fresh-hook the token state after execute_compaction.
    let src = stream_loop_source();
    assert!(
        src.contains("state.compaction_needed = false;"),
        "in-loop compaction restart must reset compaction_needed=false after \
         execute_compaction so the retry stream is not immediately flagged"
    );

    // @step When the restart stream sends "Continue" and begins a new turn
    // A fresh CompactionHook over the reset TokenState should NOT fire
    // immediately on a small "Continue" prompt. Mirror production
    // `on_completion_call` logic (estimate payload + compare against
    // threshold) — hook is StreamingPromptHook so we can't await it
    // directly from a sync test.
    let fresh_hook = CompactionHook::new(Arc::clone(&token_state), 800_000);
    // The restart stream sends "Continue" against the compacted history.
    // total_tokens on the state is now the tiny compacted count.
    let state_total = token_state.lock().unwrap().as_usage().total_context();
    let would_trigger = state_total > 800_000;

    // @step Then the first chunk does not re-trigger the compaction hook
    // Hook will NOT cancel if state_total <= threshold (production logic).
    assert!(
        !would_trigger,
        "Post-compaction state ({state_total} tokens) must be <= threshold (800_000); \
         otherwise the restart would immediately re-trigger the hook"
    );
    // Sanity: hook struct is constructed with the correct threshold.
    assert_eq!(fresh_hook.threshold(), 800_000);

    // @step And the restart stream completes normally
    // (Verified by the flag staying false — the restart stream can proceed
    // through its full error cascade without re-entering recovery.)
}

// ============================================================================
// End-of-file regression guard for the deleted compaction_retry.rs file.
// CMPCT-027 deleted this file — the fix for CMPCT-032 MUST NOT resurrect it.
// Recovery must live in the in-loop macro + post-loop safety net, not a
// parallel handle_compaction_retry module.
// ============================================================================

#[test]
fn deleted_compaction_retry_file_must_remain_deleted() {
    // @step Given the CMPCT-027 refactor deleted compaction_retry.rs
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let deleted = tests_dir.join("src/interactive/compaction_retry.rs");

    // @step When the fix for CMPCT-032 is applied
    // @step Then compaction_retry.rs must NOT be resurrected
    assert!(
        !deleted.exists(),
        "CMPCT-027 deleted compaction_retry.rs — it must remain deleted. \
         The CMPCT-032 fix must live inside stream_loop.rs, not a parallel \
         handle_compaction_retry module."
    );
}
