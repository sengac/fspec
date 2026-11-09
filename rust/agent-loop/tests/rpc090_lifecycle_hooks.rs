//! Feature: spec/features/agent-loop-lifecycle-hooks-session-start-user-prompt-submit-post-tool-use-session-end.feature
//!
//! Source-shape regression tests for RPC-090. Pins the four canonical
//! HOOK-013 lifecycle-hook call sites in the Rust agent loop so the
//! RPC-072 stub state (none of them fired) cannot silently re-emerge.
//!
//!   * `rust/agent-loop/src/agent_loop.rs` imports all three
//!     loop-side hook runners (`run_session_start`, `run_session_end`,
//!     `run_user_prompt`) and `HookMessageLevel` from
//!     `codelet_core::lifecycle_hooks`.
//!   * `run_session_start(hooks, &ctx, "startup")` fires BEFORE the
//!     main `loop {` opens — pinned via byte-offset ordering.
//!   * `run_session_end(hooks, &ctx, "exit")` fires inside the
//!     `input_rx.recv()` None arm immediately before `break;`.
//!   * `run_user_prompt(hooks, &ctx, input)` is guarded by
//!     `!hooks.user_prompt_submit.is_empty()` (avoids hook_context cost
//!     when no hooks are configured) and its block path emits the
//!     ordered tokens `set_status(SessionStatus::Idle)`,
//!     `StreamChunk::done()`, `continue;` — the `continue` is what
//!     skips the LLM for a blocked prompt.
//!   * `rust/agent-loop/src/background_output.rs` imports
//!     `run_post_tool` and invokes it inside `tokio::spawn` with the
//!     canonical 5-argument shape
//!     `&hooks_clone, &ctx, &tool_name, &tool_input, &tool_response,`.
//!   * `self.last_tool_call` mutex bridges the `StreamEvent::ToolCall`
//!     and `StreamEvent::ToolResult` arms so the post_tool_use hook
//!     receives the tool name + input that produced the result.
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no async, no tokio runtime, no `LifecycleHooks` instance is built.
//! The behavioural scenario "Lifecycle hooks fire at canonical call
//! sites" remains in `spec/features/agent-loop-lifecycle-hooks.feature`
//! as the end-to-end deferred test; RPC-090 complements that with
//! structural pinning so a regression breaks the test before CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn source(relative: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", relative]
        .iter()
        .collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn agent_loop_rs() -> String {
    source("agent_loop.rs")
}

fn lifecycle_rs() -> String {
    source("agent_loop/lifecycle.rs")
}

fn prompt_rs() -> String {
    source("agent_loop/prompt.rs")
}

fn background_output_rs() -> String {
    source("background_output.rs")
}

/// The offset of `needle` in `src`, failing with `what` when it is absent.
fn offset(src: &str, needle: &str, what: &str) -> usize {
    src.find(needle)
        .unwrap_or_else(|| panic!("{what} must contain `{needle}`"))
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: The agent loop's modules import all four loop-side hook runners by name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_the_agent_loop_modules_import_all_four_loop_side_hook_runners_by_name() {
    // @step Given I read the sources of rust/agent-loop/src/agent_loop/lifecycle.rs and rust/agent-loop/src/agent_loop/prompt.rs
    let (lifecycle, prompt) = (lifecycle_rs(), prompt_rs());
    // @step When I scan the files as strings
    // @step Then the lifecycle source must contain "use codelet_core::lifecycle_hooks::{run_session_end, run_session_start, HookMessageLevel};"
    offset(
        &lifecycle,
        "use codelet_core::lifecycle_hooks::{run_session_end, run_session_start, HookMessageLevel};",
        "agent_loop/lifecycle.rs",
    );
    // @step And the prompt source must contain "use codelet_core::lifecycle_hooks::{run_user_prompt, HookMessageLevel};"
    offset(
        &prompt,
        "use codelet_core::lifecycle_hooks::{run_user_prompt, HookMessageLevel};",
        "agent_loop/prompt.rs",
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_session_start fires with the literal phase string "startup"
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_session_start_fires_with_the_literal_phase_string_startup() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop/lifecycle.rs
    let lifecycle = lifecycle_rs();
    // @step When I scan the file as a string
    // @step Then the source must contain "run_session_start(hooks, &ctx, \"startup\").await"
    offset(
        &lifecycle,
        "run_session_start(hooks, &ctx, \"startup\").await",
        "agent_loop/lifecycle.rs",
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_session_end fires with the literal phase string "exit" before break
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_session_end_fires_with_the_literal_phase_string_exit_before_break() {
    // @step Given I read the sources of rust/agent-loop/src/agent_loop/lifecycle.rs and rust/agent-loop/src/agent_loop.rs
    let (lifecycle, agent_loop) = (lifecycle_rs(), agent_loop_rs());
    // @step When I scan the files as strings
    // @step Then the lifecycle source must contain "run_session_end(hooks, &ctx, \"exit\").await"
    offset(
        &lifecycle,
        "run_session_end(hooks, &ctx, \"exit\").await",
        "agent_loop/lifecycle.rs",
    );
    // @step And in the loop source "lifecycle::end_session_hooks(&session).await;" is immediately followed by "break;"
    let end = "lifecycle::end_session_hooks(&session).await;";
    let at = offset(&agent_loop, end, "agent_loop.rs") + end.len();
    assert!(
        agent_loop[at..].trim_start().starts_with("break;"),
        "session_end hooks must run immediately before the loop breaks"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_user_prompt is guarded by !hooks.user_prompt_submit.is_empty()
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_user_prompt_is_guarded_by_user_prompt_submit_is_empty() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop/prompt.rs
    let prompt = prompt_rs();
    // @step When I scan the file as a string
    // @step Then the source must contain "!hooks.user_prompt_submit.is_empty()"
    let guard = offset(
        &prompt,
        "!hooks.user_prompt_submit.is_empty()",
        "agent_loop/prompt.rs",
    );
    // @step And the source must contain "run_user_prompt(hooks, &ctx, input).await"
    let call = offset(
        &prompt,
        "run_user_prompt(hooks, &ctx, input).await",
        "agent_loop/prompt.rs",
    );
    // @step And the offset of "!hooks.user_prompt_submit.is_empty()" must be less than the offset of "run_user_prompt(hooks, &ctx, input).await"
    assert!(
        guard < call,
        "the empty-hooks guard must precede run_user_prompt"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: user_prompt_submit block path emits ordered Idle / done tokens and skips the turn
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_user_prompt_submit_block_path_emits_ordered_idle_done_tokens_and_skips_the_turn() {
    // @step Given I read the sources of rust/agent-loop/src/agent_loop/prompt.rs and rust/agent-loop/src/agent_loop.rs
    let (prompt, agent_loop) = (prompt_rs(), agent_loop_rs());
    // @step When I scan the files as strings
    // @step Then the prompt source must contain "!outcome.allow_prompt"
    let blocked = offset(&prompt, "!outcome.allow_prompt", "agent_loop/prompt.rs");
    // @step And after it the prompt source has "set_status(SessionStatus::Idle)", then "StreamChunk::done()", then "return false;"
    let path = &prompt[blocked..];
    let idle = offset(path, "set_status(SessionStatus::Idle)", "the block path");
    let done = offset(path, "StreamChunk::done()", "the block path");
    let skip = offset(path, "return false;", "the block path");
    assert!(
        idle < done && done < skip,
        "the block path must set Idle, then emit done, then skip the turn"
    );
    // @step And the loop source must contain "if !prompt::admit_prompt(&session, &mut pending, &mut state).await {" followed by "continue;"
    let admit = "if !prompt::admit_prompt(&session, &mut pending, &mut state).await {";
    let at = offset(&agent_loop, admit, "agent_loop.rs") + admit.len();
    assert!(
        agent_loop[at..].trim_start().starts_with("continue;"),
        "a refused prompt must skip the rest of the loop iteration"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: session_start invocation precedes the main loop opening
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_session_start_invocation_precedes_the_main_loop_opening() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop.rs
    let agent_loop = agent_loop_rs();
    // @step When I scan the file as a string
    // @step Then the source must contain "lifecycle::start_session_hooks(&session).await;"
    let start = offset(
        &agent_loop,
        "lifecycle::start_session_hooks(&session).await;",
        "agent_loop.rs",
    );
    // @step And the source must contain "loop {"
    let opening = offset(&agent_loop, "loop {", "agent_loop.rs");
    // @step And the offset of "lifecycle::start_session_hooks(&session).await;" must be less than the offset of the first "loop {"
    assert!(
        start < opening,
        "session_start hooks must run before the main loop opens"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: background_output.rs imports run_post_tool from codelet_core::lifecycle_hooks
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_background_output_rs_imports_run_post_tool_from_codelet_core_lifecycle_hooks() {
    // @step Given I read the source of rust/agent-loop/src/background_output.rs
    let src = background_output_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "use codelet_core::lifecycle_hooks::{run_post_tool, HookMessageLevel};"
    assert!(
        src.contains("use codelet_core::lifecycle_hooks::{run_post_tool, HookMessageLevel};"),
        "background_output.rs must import `run_post_tool` (and `HookMessageLevel`) from `codelet_core::lifecycle_hooks::` — this is the canonical home for the post_tool_use hook runner. Importing it from any other path means the HOOK-013 dispatcher has been bypassed (RPC-090)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_post_tool is invoked inside a tokio::spawn closure with five arguments
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_post_tool_is_invoked_inside_a_tokio_spawn_closure_with_five_arguments() {
    // @step Given I read the source of rust/agent-loop/src/background_output.rs
    let src = background_output_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "tokio::spawn(async move"
    let spawn_offset = src
        .find("tokio::spawn(async move")
        .expect("background_output.rs must wrap the post_tool_use invocation in `tokio::spawn(async move ...)` — fire-and-forget is the contract so a slow hook does not block the stream pump (RPC-090)");

    // @step And the source must contain "run_post_tool("
    let call_offset = src
        .find("run_post_tool(")
        .expect("background_output.rs must call `run_post_tool(...)` so the HOOK-013 post_tool_use hooks fire after every tool invocation (RPC-090)");

    // @step And the source must contain each of the five canonical run_post_tool arguments
    for arg in [
        "&hooks_clone,",
        "&ctx,",
        "&tool_name,",
        "&tool_input,",
        "&tool_response,",
    ] {
        assert!(
            src.contains(arg),
            "background_output.rs must call `run_post_tool` with all five canonical arguments — missing `{arg}` silently downgrades the hook's view of the tool call (RPC-090)"
        );
    }

    // @step And the offset of "tokio::spawn(async move" must be less than the offset of "run_post_tool("
    assert!(
        spawn_offset < call_offset,
        "`tokio::spawn(async move ...)` MUST appear BEFORE `run_post_tool(` — the spawn wraps the call so it runs detached. If the call appeared first it would block the stream pump on every tool result (RPC-090). spawn_offset={spawn_offset}, call_offset={call_offset}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: post_tool_use uses last_tool_call captured during StreamEvent::ToolCall
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_post_tool_use_uses_last_tool_call_captured_during_stream_event_tool_call() {
    // @step Given I read the source of rust/agent-loop/src/background_output.rs
    let src = background_output_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "last_tool_call: Arc<std::sync::Mutex<Option<(String, serde_json::Value)>>>"
    assert!(
        src.contains("last_tool_call: Arc<std::sync::Mutex<Option<(String, serde_json::Value)>>>"),
        "background_output.rs must declare `last_tool_call: std::sync::Mutex<Option<(String, serde_json::Value)>>` — this mutex is the bridge between the StreamEvent::ToolCall arm (which writes it) and the StreamEvent::ToolResult arm (which reads it to pass tool_name + tool_input to run_post_tool) (RPC-090)"
    );

    // @step And the source must contain "self.last_tool_call"
    assert!(
        src.contains("self.last_tool_call"),
        "background_output.rs must reference `self.last_tool_call` — both the ToolCall arm's write and the ToolResult arm's read go through this field (RPC-090)"
    );

    // @step And the source must contain "!hooks.post_tool_use.is_empty()"
    assert!(
        src.contains("!hooks.post_tool_use.is_empty()"),
        "background_output.rs must guard the post_tool_use spawn with `!hooks.post_tool_use.is_empty()` so we do not pay the clone + spawn cost when no hooks are configured (RPC-090)"
    );
}
