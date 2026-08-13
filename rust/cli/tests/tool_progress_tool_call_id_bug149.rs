//! Feature: spec/features/stream-loop-threads-active-tool-call-id-into-progress.feature
//!
//! BUG-149 — the stream-loop progress callback emits `ToolProgress` with an
//! EMPTY `tool_call_id`, so the TUI (which folds by EXACT id match) drops every
//! live-progress chunk. RPC-398 fixed the session-id registry key; this is the
//! SECOND, independent defect on the EMIT side.
//!
//! These are source-shape assertions in the style of
//! `rust/cli/tests/tool_progress_registration_key_rpc398.rs`. The
//! active-tool-call-id tracking mechanism does not exist yet, so the
//! bug-proving tests must FAIL now (red phase). The fix (Option A in the
//! investigation) threads the real `tool_call.id` into the progress callback:
//! set an active id on `handle_tool_call`, clear it on `handle_tool_result`,
//! and emit that id (not `""`).
//!
//! Root defect line (stream_loop.rs:474):
//!   emitter.emit_tool_progress("", "bash", chunk, is_stderr);

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

/// Read `rust/cli/src/interactive/<file>` relative to this crate's manifest.
fn cli_interactive_src(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("interactive")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Strip Rust `//` line comments so source-shape scans don't match text that
/// only appears in commentary (e.g. the old defect quoted in a doc comment).
fn strip_line_comments(src: &str) -> String {
    src.lines()
        .map(|line| match line.find("//") {
            Some(pos) => &line[..pos],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ===========================================================================
// Scenario: Progress emitted during tool execution carries the active
//           tool_call_id
//
// The progress callback closure must emit the ACTIVE tool_call_id, not the
// hard-coded empty string. This is the core bug — the emit site at
// stream_loop.rs:474 passes "".
// ===========================================================================

#[test]
fn progress_callback_does_not_emit_empty_tool_call_id() {
    // @step Given a tool call "tc-1" has started and is the active tool call
    let src = cli_interactive_src("stream_loop.rs");
    let code = strip_line_comments(&src);

    // @step When tool progress is emitted through the stream-loop progress callback
    // @step Then the emitted ToolProgress carries tool_call_id "tc-1"
    // The regression guard: the callback MUST NOT pass an empty-string
    // tool_call_id to emit_tool_progress. That empty id never matches a TUI
    // card, so all live progress is dropped.
    assert!(
        !code.contains("emit_tool_progress(\"\","),
        "stream_loop.rs must NOT emit tool progress with an empty tool_call_id \
         (`emit_tool_progress(\"\", ...)`). The empty id never matches a TUI \
         tool-call card, so live output is silently dropped (BUG-149). Thread \
         the active tool_call.id into the progress callback and emit it instead."
    );
}

#[test]
fn progress_callback_emits_the_active_tool_call_id() {
    // @step Given a tool call "tc-1" has started and is the active tool call
    let src = cli_interactive_src("stream_loop.rs");
    let code = strip_line_comments(&src);

    // Locate the progress-callback registration block.
    let reg_pos = code
        .find("set_tool_progress_callback(")
        .expect("stream_loop.rs must register a tool-progress callback");
    // The emit call lives inside the closure that follows registration.
    let emit_pos = code[reg_pos..]
        .find("emit_tool_progress(")
        .map(|rel| reg_pos + rel)
        .expect("the progress callback must call emit_tool_progress(...)");
    // Grab the first argument passed to emit_tool_progress.
    let after_open = code[emit_pos..]
        .find('(')
        .map(|p| &code[emit_pos + p + 1..])
        .expect("emit_tool_progress call must have `(`");
    let first_arg = after_open.trim_start();

    // @step When tool progress is emitted through the stream-loop progress callback
    // @step Then the emitted ToolProgress carries tool_call_id "tc-1"
    // The callback must read a threaded active-id value (some shared holder
    // captured by the closure), NOT a literal empty string.
    assert!(
        !first_arg.starts_with("\"\""),
        "the progress callback's emit_tool_progress must pass the active \
         tool_call_id as its first argument, not the empty string literal; \
         found call opening with: {:?}",
        &first_arg[..first_arg.len().min(60)]
    );
    // The threaded value should reference an "active tool call" holder so the
    // real provider id (e.g. "tc-1") reaches the wire.
    assert!(
        code.contains("active_tool_call_id"),
        "stream_loop.rs must track the active tool_call_id (e.g. an \
         `active_tool_call_id` holder) so the progress callback can emit the \
         real id (BUG-149)."
    );
}

// ===========================================================================
// Scenario: Active tool_call_id is set on ToolCall and cleared on ToolResult
//
// The active id must be SET when a ToolCall is handled and CLEARED when the
// matching ToolResult is handled. Serial tool execution within a turn makes a
// single active id unambiguous.
// ===========================================================================

#[test]
fn active_tool_call_id_set_on_tool_call_and_cleared_on_tool_result() {
    // @step Given no tool call is active
    let src = cli_interactive_src("stream_loop.rs");
    let code = strip_line_comments(&src);

    // The active-id holder must exist.
    assert!(
        code.contains("active_tool_call_id"),
        "stream_loop.rs must declare an `active_tool_call_id` holder that the \
         progress callback closure can read (BUG-149)."
    );

    // Locate the ToolCall handling site (where tool_execution_in_progress is
    // set true) and the ToolResult handling site (set false).
    let set_true_pos = code
        .find("tool_execution_in_progress = true;")
        .expect("stream_loop.rs must mark tool execution in progress on ToolCall");
    let set_false_pos = code
        .find("tool_execution_in_progress = false;")
        .expect("stream_loop.rs must clear tool execution in progress on ToolResult");

    // @step When a tool call "tc-1" starts
    // @step Then the active tool_call_id is "tc-1"
    // Around the ToolCall handling site, the active id must be SET from the
    // real tool_call id. We look for an assignment of the active id near the
    // ToolCall branch. The set must come from tool_call.id (the provider id).
    let tool_call_window_start = code[..set_true_pos]
        .rfind("handle_tool_call")
        .expect("ToolCall branch must call handle_tool_call");
    let tool_call_window = &code[tool_call_window_start..set_true_pos + 40];
    assert!(
        tool_call_window.contains("active_tool_call_id"),
        "on ToolCall, stream_loop.rs must SET the active tool_call_id from the \
         real tool_call id so live progress carries it (BUG-149). No assignment \
         to active_tool_call_id found near the ToolCall/handle_tool_call branch."
    );

    // @step When the tool call "tc-1" produces its result
    // @step Then no tool_call_id is active
    // Around the ToolResult handling site the active id must be CLEARED
    // (set back to None).
    let tool_result_window_start = code[..set_false_pos]
        .rfind("handle_tool_result")
        .expect("ToolResult branch must call handle_tool_result");
    let tool_result_window = &code[tool_result_window_start..set_false_pos + 40];
    assert!(
        tool_result_window.contains("active_tool_call_id"),
        "on ToolResult, stream_loop.rs must CLEAR the active tool_call_id \
         (back to None) so a later stray progress emit does not carry a stale \
         id (BUG-149). No clear of active_tool_call_id found near the \
         ToolResult/handle_tool_result branch."
    );
}

// ===========================================================================
// Scenario: Stray progress with no active tool call is dropped without panic
//
// When no tool is active, the active-id holder is None. The callback must
// tolerate that (no unwrap/panic) — falling back to the empty string is
// acceptable because the TUI drops it, preserving today's behaviour for that
// edge without corrupting any card.
// ===========================================================================

#[test]
fn stray_progress_with_no_active_tool_call_is_safe() {
    // @step Given no tool call is active
    let src = cli_interactive_src("stream_loop.rs");
    let code = strip_line_comments(&src);

    // The active-id holder must exist for a stray-emit to be well-defined.
    assert!(
        code.contains("active_tool_call_id"),
        "stream_loop.rs must track an `active_tool_call_id` so a stray progress \
         emit (no active tool) is well-defined and safe (BUG-149)."
    );

    // @step When tool progress is emitted through the stream-loop progress callback
    // @step Then no panic occurs
    // The progress callback must not use .unwrap()/.expect() on the active-id
    // holder — a stray emit while the holder is None must not panic.
    let reg_pos = code
        .find("set_tool_progress_callback(")
        .expect("stream_loop.rs must register a tool-progress callback");
    // Bound the closure body: from registration to the closing of the Some(...)
    // registration call. Use the next emit + a small trailing window.
    let emit_pos = code[reg_pos..]
        .find("emit_tool_progress(")
        .map(|rel| reg_pos + rel)
        .expect("the progress callback must call emit_tool_progress(...)");
    // Closure body spans from registration to just past the emit call.
    let closure_body = &code[reg_pos..emit_pos + 120];
    assert!(
        !closure_body.contains(".unwrap()") && !closure_body.contains(".expect("),
        "the progress callback closure must not panic on a stray emit — no \
         .unwrap()/.expect() on the active_tool_call_id holder (BUG-149). \
         A missing active id must degrade gracefully (fallback to empty id, \
         which the TUI drops), not panic."
    );

    // @step Then the card "tc-1" body is unchanged
    // (Guaranteed on the TUI match side: a fallback empty id matches no card;
    //  asserted behaviorally in
    //  fspec-tui/tests/tool_progress_tool_call_id_bug149.rs.)
}
