//! Feature: spec/features/agent-loop-mcp-injection-drain-mcp-injection-rx-tokio-select-arm-mcp-channel-open-flag.feature
//!
//! Source-shape regression tests for RPC-089. Pins:
//!   * The canonical `agent_loop` signature in
//!     `rust/agent-loop/src/agent_loop.rs` declares a parameter
//!     `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` — and NEVER
//!     the underscore-prefixed `_mcp_injection_rx` which marked the
//!     RPC-072 stub state where the channel was held open but never
//!     drained.
//!   * The function body declares `let mut mcp_channel_open = true;`
//!     so the closed-channel busy-loop guard is in place from turn one.
//!   * The body contains the inline-guarded select arm
//!     `result = mcp_injection_rx.recv(), if mcp_channel_open =>` so
//!     polling stops the moment the flag is flipped (MCP-001-FIX).
//!   * The body contains all three required `McpInjection` outcomes:
//!     - `Some(McpInjection::Notification(text)) =>` (process as turn)
//!     - `Some(McpInjection::SamplingRequest { params, response_tx }) =>`
//!       (V2 — acknowledged but currently rejected, must still be
//!       matched so the channel does not leak the request)
//!     - `None =>` containing `mcp_channel_open = false;`
//!   * Byte-offset ORDER: the `mut mcp_channel_open = true;` initialiser
//!     precedes the `mcp_channel_open = false;` flag flip, proving the
//!     initialiser remains the canonical starting state and the flip
//!     lives later (inside the None arm).
//!
//! These tests run in sub-milliseconds — they only read source strings,
//! no async, no tokio runtime, no MCP plumbing. The behavioural scenario
//! "MCP injection through mcp_injection_rx is processed as a turn"
//! remains in `spec/features/agent-loop-mcp-injection.feature` as the
//! end-to-end deferred test; RPC-089 complements that with structural
//! pinning so a regression breaks the test before reaching CI.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn agent_loop_rs() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let path: PathBuf = [manifest, "src", "agent_loop.rs"].iter().collect();
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

/// Walk braces forward from `abs_open` (which must be the index of an
/// opening `{`) until the matching close, returning the inclusive
/// substring.
fn brace_balanced(src: &str, abs_open: usize) -> &str {
    let bytes = src.as_bytes();
    assert!(bytes[abs_open] == b'{', "abs_open must point at `{{`");
    let mut depth: i32 = 0;
    let mut i = abs_open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[abs_open..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("matching closing brace for opener at {abs_open} not found");
}

/// Extract the body of the named function. Matches on the signature
/// substring `fn <name>(` so it works for both single-line and
/// multi-line signatures.
fn fn_body<'a>(src: &'a str, fn_name: &str) -> &'a str {
    let needle = format!("fn {fn_name}(");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("`fn {fn_name}(` not found"));
    let brace_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("opening brace for `fn {fn_name}` not found"));
    brace_balanced(src, start + brace_rel)
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: agent_loop signature declares mut mcp_injection_rx parameter
//           (no underscore prefix)
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_agent_loop_signature_declares_mut_mcp_injection_rx_parameter() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I scan the file as a string
    // (nothing to do — we just keep the source string)

    // @step Then the source must contain "mut mcp_injection_rx: mpsc::Receiver<McpInjection>"
    assert!(
        src.contains("mut mcp_injection_rx: mpsc::Receiver<McpInjection>"),
        "agent_loop signature must declare `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` so MCP server messages can flow into the per-session loop (RPC-089 / MCP-001)"
    );

    // @step And the source must NOT contain "_mcp_injection_rx"
    assert!(
        !src.contains("_mcp_injection_rx"),
        "agent_loop signature MUST NOT prefix the parameter with `_` — an underscore prefix signals the RPC-072 stub state where the channel was held open but never drained, dead-lettering every MCP server message (RPC-089)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: agent_loop body declares the mcp_channel_open initialiser
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_agent_loop_body_declares_mcp_channel_open_initialiser() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I extract the brace-balanced body of the "fn agent_loop" function
    let body = fn_body(&src, "agent_loop");

    // @step Then the function body must contain "let mut mcp_channel_open = true;"
    assert!(
        body.contains("let mut mcp_channel_open = true;"),
        "agent_loop body must declare `let mut mcp_channel_open = true;` so the tokio::select! arm can be disabled once the channel closes — preventing CPU busy-loop on a permanently-ready closed receiver (RPC-089 / MCP-001-FIX)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: tokio::select! arm is gated on if mcp_channel_open
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_tokio_select_arm_is_gated_on_if_mcp_channel_open() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I extract the brace-balanced body of the "fn agent_loop" function
    let body = fn_body(&src, "agent_loop");

    // @step Then the function body must contain "result = mcp_injection_rx.recv(), if mcp_channel_open =>"
    assert!(
        body.contains("result = mcp_injection_rx.recv(), if mcp_channel_open =>"),
        "agent_loop body must contain the inline-guarded select arm `result = mcp_injection_rx.recv(), if mcp_channel_open =>` — the `if mcp_channel_open` guard on the same arm is the busy-loop prevention contract: a closed receiver would return None immediately on every poll, causing the select to resolve instantly (RPC-089 / MCP-001-FIX)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: agent_loop body matches all three McpInjection outcomes
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_agent_loop_body_matches_all_three_mcp_injection_outcomes() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I extract the brace-balanced body of the "fn agent_loop" function
    let body = fn_body(&src, "agent_loop");

    // @step Then the function body must contain "Some(McpInjection::Notification(text)) =>"
    assert!(
        body.contains("Some(McpInjection::Notification(text)) =>"),
        "agent_loop body must contain the Notification arm `Some(McpInjection::Notification(text)) =>` so MCP server notifications are routed as a normal LLM turn (RPC-089 / MCP-001)"
    );

    // @step And the function body must contain "Some(McpInjection::SamplingRequest { params, response_tx }) =>"
    assert!(
        body.contains("Some(McpInjection::SamplingRequest { params, response_tx }) =>"),
        "agent_loop body must contain the SamplingRequest arm `Some(McpInjection::SamplingRequest {{ params, response_tx }}) =>` — even when the V1 implementation rejects sampling, the SamplingRequest variant MUST be acknowledged at this call site so the channel does not leak the request (RPC-089 / MCP-001 V2)"
    );

    // @step And the function body must contain "mcp_channel_open = false;"
    assert!(
        body.contains("mcp_channel_open = false;"),
        "agent_loop body must contain `mcp_channel_open = false;` inside the None arm of mcp_injection_rx.recv() so the closed channel cannot busy-loop the select (RPC-089 / MCP-001-FIX)"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: mcp_channel_open initialiser precedes the flag flip in
//           agent_loop
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scenario_mcp_channel_open_initialiser_precedes_the_flag_flip_in_agent_loop() {
    // @step Given I read the source of rust/agent-loop/src/agent_loop.rs
    let src = agent_loop_rs();

    // @step When I extract the brace-balanced body of the "fn agent_loop" function
    let body = fn_body(&src, "agent_loop");

    // @step Then the function body must contain "let mut mcp_channel_open = true;"
    let init_offset = body.find("let mut mcp_channel_open = true;").expect(
        "agent_loop body must declare the `let mut mcp_channel_open = true;` initialiser (RPC-089)",
    );

    // @step And the function body must contain "mcp_channel_open = false;"
    let flip_offset = body.find("mcp_channel_open = false;").expect(
        "agent_loop body must contain `mcp_channel_open = false;` inside the None arm (RPC-089)",
    );

    // @step And the offset of "let mut mcp_channel_open = true;" must be less than the offset of "mcp_channel_open = false;"
    assert!(
        init_offset < flip_offset,
        "`let mut mcp_channel_open = true;` initialiser must appear BEFORE the `mcp_channel_open = false;` flag flip inside agent_loop — proving the initialiser is the canonical starting state and the flip lives later in the loop body inside the None arm of the select (RPC-089 / MCP-001-FIX). init_offset={init_offset}, flip_offset={flip_offset}"
    );
}
