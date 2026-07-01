//! Feature: spec/features/bash-tool-progress-session-key-streaming.feature
//!
//! RPC-398: source-shape assertions pinning the stream-loop wiring fix.
//!
//! The tool-progress registry is keyed by session `Uuid` with an exact
//! HashMap lookup (BUG-126, no fallback). The regression was that
//! `codelet/cli/src/interactive/stream_loop.rs` REGISTERED the callback under
//! `Uuid::nil()` while `BashTool` EMITS under the real per-session id passed to
//! `create_rig_agent` / `BashTool::new`. Registration key (nil) != emit key
//! (real session id) => the registry lookup misses => nothing streams.
//!
//! The fix threads the real `session_id` into `run_agent_stream_internal`
//! (and the public wrappers) and registers/clears with that id. These
//! source-shape tests (in the style of `agent-loop/tests/rpc084_streaming.rs`)
//! assert:
//!   1. the registration site no longer hard-codes `Uuid::nil()`,
//!   2. the callback is registered under the threaded `session_id` param,
//!   3. the clear site uses `session_id` (not `Uuid::nil()`),
//!   4. `run_agent_stream_internal` declares a `session_id: uuid::Uuid` param.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

/// Read `codelet/cli/src/interactive/<file>` relative to this crate's manifest.
fn cli_interactive_src(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("interactive")
        .join(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ===========================================================================
// Scenario: registration site no longer hard-codes Uuid::nil()
// ===========================================================================

#[test]
fn stream_loop_registration_does_not_hardcode_uuid_nil() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let src = cli_interactive_src("stream_loop.rs");

    // @step When I inspect the tool-progress registration site
    // @step Then it does not bind `let cli_session_id = Uuid::nil();`
    assert!(
        !src.contains("let cli_session_id = Uuid::nil();"),
        "stream_loop.rs must NOT register the tool-progress callback under \
         Uuid::nil() — that key never matches the real session id BashTool \
         emits with (RPC-398). Thread the real session_id instead."
    );
}

// ===========================================================================
// Scenario: registration uses the threaded session_id parameter
// ===========================================================================

#[test]
fn stream_loop_registers_and_clears_under_threaded_session_id() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let src = cli_interactive_src("stream_loop.rs");

    // @step When I inspect the tool-progress registration and clear sites
    // @step Then the callback is registered under the threaded `session_id`
    // The registration call spans multiple lines:
    //   set_tool_progress_callback(
    //       session_id,
    //       Some(Arc::new(...
    // so we assert the call opens with `set_tool_progress_callback(` followed
    // (allowing whitespace) by the `session_id` argument and a `Some(`.
    let reg_pos = src
        .find("set_tool_progress_callback(")
        .expect("stream_loop.rs must call set_tool_progress_callback(...)");
    let reg_tail = &src[reg_pos..];
    // First occurrence should be the Some(...) registration.
    let after_open = reg_tail
        .find('(')
        .map(|p| &reg_tail[p + 1..])
        .expect("call must have `(`");
    let first_arg = after_open.trim_start();
    assert!(
        first_arg.starts_with("session_id"),
        "stream_loop.rs must register the tool-progress callback with the \
         threaded `session_id` param as the first argument; found call \
         opening with: {:?}",
        &first_arg[..first_arg.len().min(40)]
    );
    assert!(
        reg_tail
            .find("Some(")
            .is_some_and(|some_pos| some_pos < reg_tail.find(", None").unwrap_or(usize::MAX)),
        "stream_loop.rs registration call must pass `Some(...)` as the callback"
    );

    // @step And the callback is cleared under the threaded `session_id`
    assert!(
        src.contains("set_tool_progress_callback(session_id, None)"),
        "stream_loop.rs must clear the tool-progress callback with the \
         threaded `session_id` param: `set_tool_progress_callback(session_id, None)`"
    );

    // @step And no clear site clears under Uuid::nil()
    assert!(
        !src.contains("set_tool_progress_callback(Uuid::nil(), None)"),
        "stream_loop.rs must NOT clear the callback under Uuid::nil() — the \
         clear key must match the registration key (RPC-398)"
    );
}

// ===========================================================================
// Scenario: run_agent_stream_internal declares a session_id parameter
// ===========================================================================

#[test]
fn run_agent_stream_internal_declares_session_id_param() {
    // @step Given the source file codelet/cli/src/interactive/stream_loop.rs
    let src = cli_interactive_src("stream_loop.rs");

    // @step When I inspect the run_agent_stream_internal signature
    let fn_pos = src
        .find("async fn run_agent_stream_internal")
        .expect("stream_loop.rs must declare `async fn run_agent_stream_internal`");
    let sig_tail = &src[fn_pos..];
    let open = sig_tail.find('(').expect("fn signature must have `(`");
    let close = sig_tail[open..]
        .find(')')
        .map(|rel| open + rel)
        .expect("fn signature must have matching `)`");
    let sig = &sig_tail[open..=close];

    // @step Then the signature declares a `session_id: uuid::Uuid` parameter
    assert!(
        sig.contains("session_id: uuid::Uuid") || sig.contains("session_id: Uuid"),
        "run_agent_stream_internal must accept a `session_id: uuid::Uuid` \
         parameter so the real per-session id can be threaded to the \
         registration site (RPC-398); signature was:\n{sig}"
    );
}
