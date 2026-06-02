//! Feature: spec/features/agent-loop-conversation-history-session-inner-messages-round-trip-session-restore-messages-parity.feature
//!
//! RPC-081 — Source-shape regression tests for the agent loop's
//! conversation-history threading. Two scenarios pinned here:
//!
//!   1. The agent loop body MUST NOT rebuild a `vec![Message { role:
//!      MessageRole::User, ... }]` single-element history per turn (the
//!      RPC-072 stub gap from §2 of the parity attachment).
//!   2. The agent loop body MUST thread `&mut inner_session` into
//!      `codelet_cli::interactive::run_agent_stream_with_images` (the
//!      direct call site for the openai/custom-provider arms) AND as
//!      the first argument to every `run_with_provider!` macro
//!      invocation. Both pathways forward `&mut session.inner.messages`
//!      into the rig streaming engine, which is what makes
//!      conversation history round-trip across turns (canonical site
//!      codelet/cli/src/interactive/stream_loop.rs:461-471).
//!
//! These are static-source assertions only: they don't drive the
//! agent loop at runtime. The behavioural restoration tests live in
//! `codelet/sessions/tests/rpc081_restore_session_messages.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// Read a file under `codelet/agent-loop/src/`.
fn read_agent_loop_src(filename: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(filename);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "must be able to read codelet/agent-loop/src/{filename} at {}: {e}",
            path.display()
        )
    })
}

// ============================================================================
// Scenario: Source-shape — agent_loop body never constructs a single-element
// user-only history vec
// ============================================================================

#[test]
fn agent_loop_body_does_not_rebuild_single_element_user_history_vec() {
    // @step Given the source of codelet/agent-loop/src/agent_loop.rs is read into memory with Rust comments stripped
    let raw = read_agent_loop_src("agent_loop.rs");
    let src = codelet_test_helpers::dependency_rules::strip_rust_comments(&raw);

    // @step When the body is scanned for the literal pattern "vec![Message { role: MessageRole::User"
    // @step Then no match is found
    assert!(
        !src.contains("vec![Message { role: MessageRole::User"),
        "agent_loop.rs MUST NOT contain a single-element user-only history vec. \
         The RPC-072 stub at lines 78-81 rebuilt `vec![Message {{ role: MessageRole::User, ... }}]` \
         every turn — that broke chat memory. The fix threads `&mut inner_session` into the \
         rig streaming engine instead. See spec/attachments/RPC-081/agent-loop-parity-gap.md §2."
    );

    // @step And no match is found for "vec![Message { role: rig::message::MessageRole::User"
    assert!(
        !src.contains("vec![Message { role: rig::message::MessageRole::User"),
        "agent_loop.rs MUST NOT contain a fully-qualified single-element user-only history vec either"
    );
}

// ============================================================================
// Scenario: Source-shape — agent_loop body threads &mut inner_session into
// run_agent_stream_with_images
// ============================================================================

#[test]
fn agent_loop_body_threads_mut_inner_session_into_rig_streaming_engine() {
    // @step Given the source of codelet/agent-loop/src/agent_loop.rs is read into memory with Rust comments stripped
    let raw = read_agent_loop_src("agent_loop.rs");
    let src = codelet_test_helpers::dependency_rules::strip_rust_comments(&raw);

    // @step When the body is scanned for occurrences of "&mut inner_session"
    let mut_inner_count = src.matches("&mut inner_session").count();
    assert!(
        mut_inner_count >= 2,
        "expected at least 2 occurrences of `&mut inner_session` in agent_loop.rs, got {mut_inner_count}"
    );

    // @step Then at least one occurrence appears as the fourth positional argument to "codelet_cli::interactive::run_agent_stream_with_images"
    // We pin this by locating each call site of run_agent_stream_with_images
    // and asserting `&mut inner_session` appears within the call's
    // argument block (the openai-inlined arm at ~:897 and the custom-
    // provider arm at ~:996 both call it directly).
    let mut found_direct_call_with_mut_inner = false;
    let mut search_from = 0usize;
    while let Some(pos) =
        src[search_from..].find("codelet_cli::interactive::run_agent_stream_with_images")
    {
        let absolute = search_from + pos;
        // Inspect the next 600 characters — enough to cover the full
        // argument block (8 args, each on its own line in the canonical
        // formatting).
        let window_end = (absolute + 600).min(src.len());
        let window = &src[absolute..window_end];
        if window.contains("&mut inner_session") {
            found_direct_call_with_mut_inner = true;
            break;
        }
        search_from = absolute + 1;
    }
    assert!(
        found_direct_call_with_mut_inner,
        "at least one direct call to codelet_cli::interactive::run_agent_stream_with_images \
         must pass `&mut inner_session` as the chat-history slot (this is what makes \
         conversation history round-trip across turns)"
    );

    // @step And at least one occurrence appears as the first argument to a "run_with_provider!" macro invocation
    let mut found_macro_call_with_mut_inner = false;
    let mut search_from = 0usize;
    while let Some(pos) = src[search_from..].find("run_with_provider!(") {
        let absolute = search_from + pos;
        let window_end = (absolute + 200).min(src.len());
        let window = &src[absolute..window_end];
        if window.contains("&mut inner_session") {
            found_macro_call_with_mut_inner = true;
            break;
        }
        search_from = absolute + 1;
    }
    assert!(
        found_macro_call_with_mut_inner,
        "at least one `run_with_provider!(...)` invocation in agent_loop.rs must pass \
         `&mut inner_session` as the first argument — the macro forwards that slot into \
         run_agent_stream_with_images so chat history threads through to the LLM"
    );
}
