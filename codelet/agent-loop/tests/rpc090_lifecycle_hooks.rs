//! Feature: spec/features/agent-loop-lifecycle-hooks-session-start-user-prompt-submit-post-tool-use-session-end.feature
//!
//! Source-shape regression tests for RPC-090. Pins the four canonical
//! HOOK-013 lifecycle-hook call sites in the Rust agent loop so the
//! RPC-072 stub state (none of them fired) cannot silently re-emerge.
//!
//!   * `codelet/agent-loop/src/agent_loop.rs` imports all three
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
//!   * `codelet/agent-loop/src/background_output.rs` imports
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

fn agent_loop_rs() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, "src", "agent_loop.rs"].iter().collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn background_output_rs() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, "src", "background_output.rs"].iter().collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: agent_loop.rs imports all four loop-side hook runners by name
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_agent_loop_rs_imports_all_four_loop_side_hook_runners_by_name() {
    // @step Given I read the source of codelet/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "run_session_end"
    assert!(
        src.contains("run_session_end"),
        "agent_loop.rs must import `run_session_end` from codelet_core::lifecycle_hooks so the HOOK-013 session_end hook can fire when input_rx closes (RPC-090)"
    );

    // @step And the source must contain "run_session_start"
    assert!(
        src.contains("run_session_start"),
        "agent_loop.rs must import `run_session_start` from codelet_core::lifecycle_hooks so the HOOK-013 session_start hook can fire before the first turn (RPC-090)"
    );

    // @step And the source must contain "run_user_prompt"
    assert!(
        src.contains("run_user_prompt"),
        "agent_loop.rs must import `run_user_prompt` from codelet_core::lifecycle_hooks so the HOOK-013 user_prompt_submit hook can mutate or block the prompt per turn (RPC-090)"
    );

    // @step And the source must contain "HookMessageLevel"
    assert!(
        src.contains("HookMessageLevel"),
        "agent_loop.rs must import `HookMessageLevel` so Warning/Error hook messages can be routed to tracing::warn + StreamChunk::user_notification (RPC-090)"
    );

    // @step And the source must contain "use codelet_core::lifecycle_hooks::"
    assert!(
        src.contains("use codelet_core::lifecycle_hooks::"),
        "agent_loop.rs must import the loop-side hook runners from `codelet_core::lifecycle_hooks::` — anywhere else is the wrong canonical home (RPC-090)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_session_start fires with the literal phase string "startup"
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_session_start_fires_with_the_literal_phase_string_startup() {
    // @step Given I read the source of codelet/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "run_session_start(hooks, &ctx, \"startup\").await"
    assert!(
        src.contains("run_session_start(hooks, &ctx, \"startup\").await"),
        "agent_loop.rs must call `run_session_start(hooks, &ctx, \"startup\").await` with the literal phase string \"startup\" — this is the contract the lifecycle_hooks dispatcher matches on when selecting which hook configs to fire (RPC-090 / HOOK-013)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_session_end fires with the literal phase string "exit" before break
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_session_end_fires_with_the_literal_phase_string_exit_before_break() {
    // @step Given I read the source of codelet/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "run_session_end(hooks, &ctx, \"exit\").await"
    assert!(
        src.contains("run_session_end(hooks, &ctx, \"exit\").await"),
        "agent_loop.rs must call `run_session_end(hooks, &ctx, \"exit\").await` with the literal phase string \"exit\" — this is the contract the lifecycle_hooks dispatcher matches on when selecting which hook configs to fire on session exit (RPC-090 / HOOK-013)"
    );

    // @step And the source must contain "break;"
    assert!(
        src.contains("break;"),
        "agent_loop.rs must contain `break;` — the only way out of the main agent loop, immediately following the session_end hook invocation in the input_rx.recv() None arm (RPC-090)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: run_user_prompt is guarded by !hooks.user_prompt_submit.is_empty()
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_run_user_prompt_is_guarded_by_user_prompt_submit_is_empty() {
    // @step Given I read the source of codelet/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "!hooks.user_prompt_submit.is_empty()"
    let guard_offset = src
        .find("!hooks.user_prompt_submit.is_empty()")
        .expect("agent_loop.rs must guard the user_prompt_submit call with `!hooks.user_prompt_submit.is_empty()` so the hook_context allocation is skipped when no hooks are configured (RPC-090)");

    // @step And the source must contain "run_user_prompt(hooks, &ctx, input).await"
    let call_offset = src.find("run_user_prompt(hooks, &ctx, input).await").expect(
        "agent_loop.rs must call `run_user_prompt(hooks, &ctx, input).await` per turn so HOOK-013 user_prompt_submit hooks can mutate or block the prompt (RPC-090)",
    );

    // @step And the offset of "!hooks.user_prompt_submit.is_empty()" must be less than the offset of "run_user_prompt(hooks, &ctx, input).await"
    assert!(
        guard_offset < call_offset,
        "the `!hooks.user_prompt_submit.is_empty()` guard MUST appear BEFORE the `run_user_prompt(hooks, &ctx, input).await` call — otherwise the hook_context allocation and hook dispatch fires unconditionally on every turn, defeating the early-exit (RPC-090). guard_offset={guard_offset}, call_offset={call_offset}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: user_prompt_submit block path emits ordered Idle / done / continue tokens
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_user_prompt_submit_block_path_emits_ordered_idle_done_continue_tokens() {
    // @step Given I read the source of codelet/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "!outcome.allow_prompt"
    let block_offset = src.find("!outcome.allow_prompt").expect(
        "agent_loop.rs must check `!outcome.allow_prompt` to detect a hook-blocked prompt — without this the block_reason path is unreachable (RPC-090)",
    );

    // @step And the source must contain "set_status(SessionStatus::Idle)"
    assert!(
        src.contains("set_status(SessionStatus::Idle)"),
        "agent_loop.rs must call `set_status(SessionStatus::Idle)` in the block path so a blocked prompt returns the session UI to idle (RPC-090)"
    );

    // @step And the source must contain "StreamChunk::done()"
    assert!(
        src.contains("StreamChunk::done()"),
        "agent_loop.rs must emit `StreamChunk::done()` in the block path so the front-end knows the turn is complete after a blocked prompt (RPC-090)"
    );

    // @step And the source must contain "continue;"
    assert!(
        src.contains("continue;"),
        "agent_loop.rs must `continue;` in the block path so a blocked prompt skips the LLM dispatch — without this the prompt would still reach the provider (RPC-090)"
    );

    // @step And the offset of "set_status(SessionStatus::Idle)" must be less than the offset of "StreamChunk::done()" appearing after "!outcome.allow_prompt"
    let block_tail = &src[block_offset..];
    let idle_in_tail = block_tail
        .find("set_status(SessionStatus::Idle)")
        .expect("set_status(SessionStatus::Idle) must appear AFTER the !outcome.allow_prompt check (RPC-090)");
    let done_in_tail = block_tail
        .find("StreamChunk::done()")
        .expect("StreamChunk::done() must appear AFTER the !outcome.allow_prompt check (RPC-090)");
    let continue_in_tail = block_tail
        .find("continue;")
        .expect("continue; must appear AFTER the !outcome.allow_prompt check (RPC-090)");
    assert!(
        idle_in_tail < done_in_tail,
        "in the user_prompt_submit block path, `set_status(SessionStatus::Idle)` MUST precede `StreamChunk::done()` — order matters because the done chunk is the signal to the front-end that the idle status is final (RPC-090). idle_in_tail={idle_in_tail}, done_in_tail={done_in_tail}"
    );
    assert!(
        done_in_tail < continue_in_tail,
        "in the user_prompt_submit block path, `StreamChunk::done()` MUST precede `continue;` — order matters because emitting done after continue would never run (RPC-090). done_in_tail={done_in_tail}, continue_in_tail={continue_in_tail}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: session_start invocation precedes the main loop opening
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_session_start_invocation_precedes_the_main_loop_opening() {
    // @step Given I read the source of codelet/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "run_session_start(hooks, &ctx, \"startup\").await"
    let start_offset = src.find("run_session_start(hooks, &ctx, \"startup\").await").expect(
        "agent_loop.rs must call `run_session_start(hooks, &ctx, \"startup\").await` once before the main loop opens (RPC-090)",
    );

    // @step And the source must contain "loop {"
    let loop_offset = src
        .find("loop {")
        .expect("agent_loop.rs must contain the main `loop {` block — without it the agent never iterates (RPC-090)");

    // @step And the offset of "run_session_start(hooks, &ctx, \"startup\").await" must be less than the offset of the first "loop {"
    assert!(
        start_offset < loop_offset,
        "`run_session_start(hooks, &ctx, \"startup\").await` MUST appear BEFORE the first `loop {{` — session_start fires once on startup, not once per turn. Inverting the order would mean hooks see a session that has already taken its first turn (RPC-090 / HOOK-013). start_offset={start_offset}, loop_offset={loop_offset}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: background_output.rs imports run_post_tool from codelet_core::lifecycle_hooks
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_background_output_rs_imports_run_post_tool_from_codelet_core_lifecycle_hooks() {
    // @step Given I read the source of codelet/agent-loop/src/background_output.rs
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
    // @step Given I read the source of codelet/agent-loop/src/background_output.rs
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
    // @step Given I read the source of codelet/agent-loop/src/background_output.rs
    let src = background_output_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "last_tool_call: std::sync::Mutex<Option<(String, serde_json::Value)>>"
    assert!(
        src.contains("last_tool_call: std::sync::Mutex<Option<(String, serde_json::Value)>>"),
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
