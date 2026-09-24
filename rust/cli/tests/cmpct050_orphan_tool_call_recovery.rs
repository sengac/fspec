#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/orphan-tool-call-defeats-execute-compaction-trailing-user-pop-removes-a-toolresult-after-the-cmpct-029-cleanup-ran.feature
//!
//! CMPCT-050: Orphan tool_call defeats execute_compaction.
//!
//! Regression: on Path C the CMPCT-029 cleanup (reconcile + drain +
//! inject_synthetic) ran BEFORE `begin_compaction_recovery`, whose
//! trailing-User pop (`matches!(last, Message::User { .. })`) removed a
//! `User(ToolResult)` and re-orphaned its tool_call — so
//! `execute_compaction`'s defensive guard refused and the user saw
//! "Compaction failed: execute_compaction refuses to proceed: 1 orphan
//! tool_call(s)... - will retry on next turn".
//!
//! Fix contract pinned here:
//! 1. The pop must NOT remove a trailing `User(ToolResult)` (mid-turn
//!    conversation state) — plain text prompts are still popped.
//! 2. The orphan safety net (synthetic cancelled tool_results) must run at
//!    the end of `begin_compaction_recovery` — AFTER the pop and the
//!    partial-text flush — so EVERY entry path (B/C/D/032/overflow) leaves
//!    `session.messages` tool-pair-clean before `execute_compaction`.
//! 3. The manual `/compact` twins (sessions `compact_session`, NAPI
//!    `session_compact`) must sanitize before `execute_compaction` so a
//!    persisted orphan cannot wedge the session.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use codelet_cli::interactive::begin_compaction_recovery;
use codelet_cli::interactive::output::{StreamEvent, StreamOutput};
use codelet_cli::interactive_helpers::{
    execute_compaction, inject_synthetic_tool_results_for_orphans, validate_no_orphan_tool_calls,
};
use codelet_cli::session::Session;
use codelet_core::{StreamingTokenDisplay, TokenState};
use rig::message::{AssistantContent, Message, ToolCall, ToolFunction, UserContent};
use rig::OneOrMany;

// ============================================================================
// Shared fixtures
// ============================================================================

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

/// Process-wide tempdir kept alive for the lifetime of the test binary —
/// `execute_compaction` fans out into debug-capture which requires
/// `set_data_directory` (same pattern as compaction_tool_call_preservation_test).
static TEST_DATA_DIR: OnceLock<tempfile::TempDir> = OnceLock::new();

fn ensure_test_data_dir() {
    TEST_DATA_DIR.get_or_init(|| {
        let temp_dir = tempfile::tempdir().expect("failed to create temp directory");
        codelet_common::set_data_directory(temp_dir.path().to_path_buf())
            .expect("failed to set data directory");
        temp_dir
    });
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

fn make_tool_call(id: &str, call_id: Option<&str>, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        call_id: call_id.map(ToString::to_string),
        function: ToolFunction {
            name: name.to_string(),
            arguments: serde_json::json!({}),
        },
        signature: None,
        additional_params: None,
    }
}

fn assistant_tool_call_message(tc: ToolCall) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::ToolCall(tc)),
    }
}

fn user_tool_result_message(id: &str, call_id: Option<&str>, body: &str) -> Message {
    let content = OneOrMany::one(rig::message::ToolResultContent::Text(rig::message::Text {
        text: body.to_string(),
    }));
    let user_content = match call_id {
        Some(cid) => UserContent::tool_result_with_call_id(id, cid.to_string(), content),
        None => UserContent::tool_result(id, content),
    };
    Message::User {
        content: OneOrMany::one(user_content),
    }
}

fn user_text_message(text: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(text)),
    }
}

fn has_tool_result(msg: &Message) -> bool {
    matches!(
        msg,
        Message::User { content }
            if content
                .iter()
                .any(|c| matches!(c, UserContent::ToolResult(_)))
    )
}

fn is_tool_result_with_call_id(msg: &Message, call_id: &str) -> bool {
    matches!(
        msg,
        Message::User { content }
            if content.iter().any(|c| matches!(
                c,
                UserContent::ToolResult(tr) if tr.call_id.as_deref() == Some(call_id)
            ))
    )
}

fn is_plain_user_text(msg: &Message, text: &str) -> bool {
    matches!(
        msg,
        Message::User { content }
            if content.iter().any(|c| matches!(
                c,
                UserContent::Text(t) if t.text == text
            ))
    )
}

/// Read a workspace-relative source file. Tests run with the crate dir as
/// CWD, so walk up from `CARGO_MANIFEST_DIR` (rust/cli) two levels to the
/// repo root — same pattern as `tool_progress_registration_key_rpc398.rs`.
fn read_repo_file(relative: &str) -> String {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate dir (rust/cli) must have two parents (repo root)");
    let path = repo_root.join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Line number (1-based) of the first line containing `needle`.
fn line_number_of(src: &str, needle: &str) -> Option<usize> {
    src.lines()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .map(|(idx, _)| idx + 1)
}

/// Extract the body of `fn_name` (from its definition line to the next
/// top-level item, or EOF), for ordering assertions.
fn function_body(src: &str, fn_name: &str) -> String {
    let start =
        line_number_of(src, &format!("pub fn {fn_name}")).expect("function {fn_name} must exist");
    let lines: Vec<&str> = src.lines().collect();
    let body_start = start - 1; // 0-based
    let end_rel = lines
        .iter()
        .skip(body_start + 1)
        .position(|l| l.starts_with("pub ") || l.starts_with("pub("))
        .unwrap_or(lines.len() - body_start);
    lines[body_start..body_start + end_rel].join("\n")
}

// ============================================================================
// Scenario 1: begin_compaction_recovery preserves a trailing
// User(ToolResult) at the tail of session.messages
// ============================================================================

#[test]
fn begin_compaction_recovery_preserves_trailing_user_tool_result() {
    // @step Given a session whose messages end with an Assistant(ToolCall) for call X and the matching User(ToolResult) for call X
    let mut session = fresh_session();
    let call = make_tool_call("toolu_cmpct050", Some("call_cmpct050_x"), "read_file");
    session.messages.push(user_text_message("do something"));
    session.messages.push(assistant_tool_call_message(call));
    session.messages.push(user_tool_result_message(
        "toolu_cmpct050",
        Some("call_cmpct050_x"),
        "ok",
    ));
    let len_before = session.messages.len();

    // Precondition mirrors the bug: the CMPCT-029 cleanup has already run —
    // the list is clean. The pop must not break that.
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "precondition: session starts tool-pair-clean"
    );

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::new();

    // @step When begin_compaction_recovery runs with pop_user_prompt=true
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("helper must succeed");

    // @step Then the trailing User(ToolResult) message is preserved in session.messages
    assert!(
        has_tool_result(session.messages.last().expect("tail must exist")),
        "CMPCT-050: a trailing User(ToolResult) must NOT be popped — it is \
         mid-turn conversation state, not an unconsumed prompt"
    );

    // @step And validate_no_orphan_tool_calls reports zero orphan call_ids for the session
    assert!(
        is_tool_result_with_call_id(
            session.messages.last().expect("tail must exist"),
            "call_cmpct050_x"
        ),
        "the matching tool result for call_cmpct050_x must still be at the tail"
    );
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "CMPCT-050: begin_compaction_recovery must leave session.messages \
         tool-pair-clean (the pop must not re-orphan a tool call)"
    );
    assert_eq!(
        session.messages.len(),
        len_before,
        "no message may be removed when the tail is a User(ToolResult)"
    );
}

// ============================================================================
// Scenario 2 (control): A trailing plain User text prompt is still popped
// (no behavior change)
// ============================================================================

#[test]
fn trailing_plain_user_text_prompt_is_still_popped() {
    // @step Given a session whose messages end with a plain User text prompt that the API has not consumed
    let mut session = fresh_session();
    session
        .messages
        .push(user_text_message("earlier conversation"));
    session.messages.push(user_text_message("Hi"));
    let len_before = session.messages.len();

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::new();

    // @step When begin_compaction_recovery runs with pop_user_prompt=true
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("helper must succeed");

    // @step Then the trailing User text prompt is removed from session.messages
    // @step And no User(ToolResult) message is altered or removed
    assert_eq!(
        session.messages.len(),
        len_before - 1,
        "the trailing plain text prompt must still be popped (original behavior)"
    );
    assert!(
        is_plain_user_text(
            session.messages.last().expect("tail must exist"),
            "earlier conversation"
        ),
        "after the pop the tail must be the earlier User text message"
    );
}

// ============================================================================
// Scenario 3: PromptCancelled firing right after a tool result lets
// compaction resume without a failure notification
// ============================================================================

#[tokio::test]
async fn prompt_cancelled_after_tool_result_compacts_cleanly() {
    ensure_test_data_dir();

    // @step Given a sub-agent stream in which the CompactionHook cancels on the usage update immediately after a tool result
    // (Expressed as: the Path C state immediately after the CMPCT-029
    // cleanup — session tail is a complete tool pair.)

    // @step And session.messages ends with the matching Assistant(ToolCall) and User(ToolResult) pair
    let mut session = fresh_session();
    let call = make_tool_call(
        "toolu_660169",
        Some("call_66016913094b485a8b5b189d"),
        "deep_search",
    );
    session
        .messages
        .push(user_text_message("research the codebase"));
    session.messages.push(assistant_tool_call_message(call));
    session.messages.push(user_tool_result_message(
        "toolu_660169",
        Some("call_66016913094b485a8b5b189d"),
        "research findings",
    ));

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::from("partial analysis");

    // Run the Path C sequence exactly as stream_loop.rs does:
    // begin_compaction_recovery(..., true) then execute_compaction.
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("recovery must succeed");

    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // @step When the compaction-recovery path runs and execute_compaction builds the DAG instruction
    let result = execute_compaction(&mut session, compaction_in_progress, Some("Continue")).await;

    // @step Then no "Compaction failed" notification is emitted
    assert_eq!(
        output.count(|e| matches!(e, StreamEvent::CompactionFailed { .. })),
        0,
        "no CompactionFailed event may be emitted"
    );

    // @step And execute_compaction succeeds instead of refusing on the orphan guard
    assert!(
        result.is_ok(),
        "CMPCT-050: compaction after a tool-result-tail cancel must SUCCEED, \
         got: {result:?} — the trailing User(ToolResult) must not have been \
         popped (or the safety net must have closed the orphan)"
    );

    // @step And the post-compaction session is tool-pair-clean for the next turn
    // (In-view DAG compaction clears the compactable messages; the result is
    // folded into the DAG via SessionSearch. What must NOT happen is an
    // orphan leaking into the next turn's request.)
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "post-compaction session must be tool-pair-clean for the next turn"
    );
}

// ============================================================================
// Scenario 4: Orphan safety net closes orphans after all mutations on
// every entry path
// ============================================================================

#[test]
fn safety_net_closes_orphans_after_all_mutations() {
    // @step Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    let mut session = fresh_session();
    let call = make_tool_call("toolu_orphan", Some("call_orphan_prior"), "bash");
    session.messages.push(user_text_message("prior prompt"));
    session.messages.push(assistant_tool_call_message(call));
    let orphans = validate_no_orphan_tool_calls(&session.messages).err();
    assert_eq!(
        orphans.as_ref().map(Vec::len),
        Some(1),
        "precondition: exactly one orphan tool_call must be present"
    );

    let token_state = fresh_token_state();
    let display = StreamingTokenDisplay::new(0, 0, 0, 0);
    let output = RecordingOutput::new();
    let mut assistant_text = String::new();

    // @step When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    // (Every entry path funnels through begin_compaction_recovery, so
    // running it with pop_user_prompt=true exercises the choke point all
    // of them share.)
    begin_compaction_recovery(
        &mut session,
        &token_state,
        &display,
        &mut assistant_text,
        &output,
        true,
    )
    .expect("helper must succeed");

    // @step Then the orphan call receives a synthetic User(ToolResult) marked "cancelled_by_context_limit"
    assert!(
        session
            .messages
            .iter()
            .any(|m| is_tool_result_with_call_id(m, "call_orphan_prior")),
        "a User(ToolResult) for call_orphan_prior must have been injected"
    );

    // @step And execute_compaction's orphan guard passes for that path
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_ok(),
        "CMPCT-050: after begin_compaction_recovery the orphan must be closed \
         (synthetic cancelled tool_result injected) so execute_compaction's \
         guard passes on every entry path"
    );
}

// Source-shape pins: every in-loop compaction entry path must funnel through
// begin_compaction_recovery (the choke point that now owns the safety net)
// and none may call execute_compaction directly.
#[test]
fn all_in_loop_entry_paths_funnel_through_the_choke_point() {
    let src = read_repo_file("rust/cli/src/interactive/stream_loop.rs");

    // @step Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    // (Structural: the guarantee exists because the wiring is shared.)

    // @step When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    // The in-loop restart macro is the single place compaction executes
    // for paths B, C, D, 032-in-loop, and overflow; the CMPCT-032 POST-LOOP
    // safety net (stream_loop.rs, after the loop breaks) is the second site.
    // Both funnel through begin_compaction_recovery FIRST — that is the
    // contract this test pins.
    let real_call_sites: Vec<String> = src
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            trimmed.contains("execute_compaction_and_capture_events(")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("///")
        })
        .map(|l| l.trim().to_string())
        .collect();
    assert_eq!(
        real_call_sites.len(),
        2,
        "stream_loop.rs must have exactly two call sites for \
         execute_compaction_and_capture_events — the in_loop_compaction_restart! \
         macro (paths B/C/D/032/overflow) and the CMPCT-032 POST-LOOP safety \
         net. Both funnel through begin_compaction_recovery first. Sites: \
         {real_call_sites:?}"
    );
    // And the CMPCT-032 POST-LOOP arm must call begin_compaction_recovery
    // (post_loop_text) BEFORE execute_compaction_and_capture_events — the
    // in-loop macro's caller contract pins the other site (its doc
    // comment requires begin_compaction_recovery upstream).
    let post_loop_helper = line_number_of(&src, "let mut post_loop_text = String::new();")
        .expect("the CMPCT-032 POST-LOOP arm must exist");
    let post_loop_exec = line_number_of(
        &src,
        "if let Err(e) = super::recovery_compaction::execute_compaction_and_capture_events(",
    )
    .expect("the CMPCT-032 POST-LOOP execution site must exist");
    assert!(
        post_loop_helper < post_loop_exec,
        "the CMPCT-032 POST-LOOP arm must call begin_compaction_recovery \
         BEFORE execute_compaction_and_capture_events — helper at line \
         {post_loop_helper}, exec at line {post_loop_exec}"
    );
    // Exactly ONE direct `execute_compaction(` call site may exist in the
    // stream loop: Path A (pre-prompt compaction), which is structurally
    // separate from the B/C/D/032/overflow paths and has no
    // begin_compaction_recovery choke point — it compensates with a
    // preflight orphan injection instead (CMPCT-050).
    let direct_calls: Vec<String> = src
        .lines()
        .filter(|l| {
            l.trim().contains("execute_compaction(session,")
                || l.trim().contains("execute_compaction(&mut session,")
        })
        .map(|l| l.trim().to_string())
        .collect();
    assert_eq!(
        direct_calls.len(),
        1,
        "CMPCT-050: only Path A (pre-prompt compaction) may call \
         execute_compaction directly in stream_loop.rs — every other entry \
         path must go through begin_compaction_recovery first. Sites: \
         {direct_calls:?}"
    );

    // And that direct call (Path A) must be preceded by the preflight
    // orphan injection.
    let path_a_inject = line_number_of(
        &src,
        "closed {injected} orphan tool_call(s) before pre-prompt compaction",
    )
    .expect(
        "CMPCT-050: Path A must run inject_synthetic_tool_results_for_orphans \
             before execute_compaction (preflight)",
    );
    let path_a_exec = line_number_of(
        &src,
        "match execute_compaction(session, compaction_in_progress.clone(), Some(prompt)).await",
    )
    .expect("Path A (pre-prompt compaction) must still call execute_compaction");
    assert!(
        path_a_inject < path_a_exec,
        "the Path A preflight orphan injection must run BEFORE \
         execute_compaction — injection at line {path_a_inject}, exec at \
         line {path_a_exec}"
    );
}

#[test]
fn begin_compaction_recovery_owns_the_safety_net_after_mutation() {
    let src = read_repo_file("rust/cli/src/interactive/recovery_compaction.rs");

    // @step Given a session whose persisted messages contain an orphan Assistant(ToolCall) from a prior turn
    let body = function_body(&src, "begin_compaction_recovery");

    // @step When any compaction-recovery entry path (prompt-too-long, PromptCancelled, Gemini continuation, clean-exit, or context-overflow) runs the safety net before execute_compaction
    // The synthetic injection must be invoked from within
    // begin_compaction_recovery — after the pop + partial-text flush — so
    // the post-mutation state is the one guaranteed clean.
    assert!(
        body.contains("inject_synthetic_tool_results_for_orphans"),
        "CMPCT-050: begin_compaction_recovery must run the orphan safety net \
         (inject_synthetic_tool_results_for_orphans) so EVERY entry path \
         (B/C/D/032/overflow) leaves session.messages tool-pair-clean"
    );

    // And it must run AFTER the pop step, not before it.
    let pop_pos = body
        .find("session.messages.pop()")
        .expect("the trailing-User pop must still exist in begin_compaction_recovery");
    let inject_pos = body
        .find("inject_synthetic_tool_results_for_orphans")
        .expect("safety net must be present");
    assert!(
        inject_pos > pop_pos,
        "CMPCT-050: the safety net must run AFTER any pop/mutation — \
         cleanup-before-pop is exactly the ordering this bug fixes"
    );

    // @step Then the orphan call receives a synthetic User(ToolResult) marked "cancelled_by_context_limit"
    // (Guaranteed by the unit-level test above; the ordering pin is here.)

    // @step And execute_compaction's orphan guard passes for that path
    // (Verified end-to-end by prompt_cancelled_after_tool_result_compacts_cleanly.)
}

// ============================================================================
// Scenario 5: Manual /compact recovers a persisted orphan instead of failing
// ============================================================================

#[tokio::test]
async fn manual_compact_recovers_persisted_orphan() {
    ensure_test_data_dir();

    // @step Given a session file whose messages contain an orphan tool_call from a previously failed compaction
    // (Expressed as: a live Session with a persisted orphan — exactly what
    // compact_session finds after a failed in-loop compaction.)
    let mut session = fresh_session();
    let call = make_tool_call("toolu_persisted", Some("call_persisted_01"), "bash");
    session.messages.push(user_text_message("earlier prompt"));
    session.messages.push(assistant_tool_call_message(call));
    session.messages.push(user_text_message("follow-up prompt"));
    assert!(
        validate_no_orphan_tool_calls(&session.messages).is_err(),
        "precondition: session must start with one persisted orphan"
    );

    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // @step When the user runs /compact
    // (The manual /compact twins — codelet-sessions compact_session and the
    // NAPI session_compact — must close persisted orphans with
    // inject_synthetic_tool_results_for_orphans before calling
    // execute_compaction. We exercise the shared helper contract they must
    // call.)
    inject_synthetic_tool_results_for_orphans(&mut session.messages);
    let result = execute_compaction(&mut session, compaction_in_progress, None).await;

    // @step Then compaction succeeds and DAG construction begins
    // @step And no "Compaction failed: execute_compaction refuses to proceed" error is returned to the user
    assert!(
        result.is_ok(),
        "CMPCT-050: manual /compact on a session with a persisted orphan must \
         SUCCEED after preflight sanitize, got: {result:?}"
    );
    // The /compact wiring itself is pinned structurally in
    // manual_compact_twins_sanitize_before_execute below.
}

#[test]
fn manual_compact_twins_sanitize_before_execute() {
    // @step Given a session file whose messages contain an orphan tool_call from a previously failed compaction
    // @step When the user runs /compact
    // Both manual /compact twins must close tool-pair orphans (via
    // inject_synthetic_tool_results_for_orphans) BEFORE
    // execute_compaction, so a persisted orphan cannot wedge the session.

    let sessions_src = read_repo_file("rust/sessions/src/handle_impl.rs");
    let napi_src = read_repo_file("rust/napi/src/session_bindings.rs");

    let sanitize = "inject_synthetic_tool_results_for_orphans";

    let sessions_call = line_number_of(&sessions_src, sanitize).expect(
        "CMPCT-050: codelet-sessions compact_session must call \
                 inject_synthetic_tool_results_for_orphans before execute_compaction",
    );
    let sessions_exec = line_number_of(&sessions_src, "execute_compaction(&mut inner")
        .expect("compact_session must still call execute_compaction");
    assert!(
        sessions_call < sessions_exec,
        "the preflight orphan-closing must run BEFORE execute_compaction in \
         compact_session (sanitize at line {sessions_call}, execute at \
         line {sessions_exec})"
    );

    let napi_call = line_number_of(&napi_src, sanitize).expect(
        "CMPCT-050: NAPI session_compact must call \
                 inject_synthetic_tool_results_for_orphans before execute_compaction",
    );
    let napi_exec = line_number_of(&napi_src, "execute_compaction(&mut inner")
        .expect("session_compact must still call execute_compaction");
    assert!(
        napi_call < napi_exec,
        "the preflight orphan-closing must run BEFORE execute_compaction in \
         session_compact (sanitize at line {napi_call}, execute at line \
         {napi_exec})"
    );
}

// ============================================================================
// Scenario 6: execute_compaction still refuses when an orphan survives
// with no recovery performed (guard unchanged)
// ============================================================================

#[tokio::test]
async fn execute_compaction_still_refuses_unrecovered_orphan() {
    ensure_test_data_dir();

    // @step Given a session whose messages contain an orphan Assistant(ToolCall) with no matching User(ToolResult)
    let mut session = fresh_session();
    let call = make_tool_call("toolu_guard", Some("call_guard_still"), "bash");
    session.messages.push(user_text_message("prompt"));
    session.messages.push(assistant_tool_call_message(call));
    let messages_len_before = session.messages.len();

    // @step And no reconciliation, drain, or synthetic injection has been performed
    // (We call execute_compaction directly — bypassing all recovery paths.)

    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // @step When execute_compaction is invoked directly
    let result = execute_compaction(&mut session, compaction_in_progress.clone(), None).await;

    // @step Then it returns an error listing the orphan call_id
    assert!(
        result.is_err(),
        "the defensive guard must still refuse on an unrecovered orphan"
    );
    let err = result.err().unwrap();
    assert!(
        err.to_string().contains("call_guard_still"),
        "the refusal error must list the offending call_id, got: {err}"
    );
    assert_eq!(
        session.messages.len(),
        messages_len_before,
        "no compaction instruction may be pushed when the guard refuses"
    );

    // @step And the defensive guard behavior of CMPCT-029 is unchanged
    // The guard fires before any mutation, so the flag is untouched.
    assert!(!compaction_in_progress.load(Ordering::SeqCst));
}
