//! Feature: spec/features/agent-loop-token-tracking.feature
//!
//! RPC-086 (RPC-072 family): token tracking parity. The canonical
//! NAPI agent loop, on every `StreamEvent::Tokens` carried up through
//! the rig streaming engine, calls
//! `session.update_tokens(input, output)` plus optional
//! `session.update_reasoning_tokens(reasoning)` on `BackgroundSession`
//! and emits a `StreamChunk::TokenUpdate` carrying a populated
//! `TokenTracker`. It also translates `StreamEvent::ContextFill` into
//! `StreamChunk::ContextFillUpdate(ContextFillInfo { ... })`.
//!
//! After the RPC-072/RPC-080/RPC-081 ports the same plumbing lives in
//! the NAPI-free `codelet-agent-loop` crate. This test file pins the
//! contract via:
//!
//!   1. Structural source-string assertions over the
//!      `StreamEvent::Tokens` arm of `BackgroundOutput::emit` in
//!      `rust/agent-loop/src/background_output.rs`.
//!   2. Structural assertions over the `StreamEvent::ContextFill` arm
//!      in the same file.
//!   3. An integration round-trip on `BackgroundSession::update_tokens`
//!      / `update_reasoning_tokens` / `get_tokens`.
//!   4. A census of `codelet_rpc_types::StreamChunk::{TokenUpdate,
//!      ContextFillUpdate}` variants + their constructors.
//!   5. A census of `codelet_cli::interactive::{TokenInfo,
//!      ContextFillInfo, StreamEvent}` source types.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

// ===========================================================================
// Helpers
// ===========================================================================

fn agent_loop_src(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file)
}

fn read_source(file: &str) -> String {
    let path = agent_loop_src(file);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

fn read_workspace_source(rel: &str) -> String {
    // CARGO_MANIFEST_DIR = rust/agent-loop; walk up two parents to repo root.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

/// Extract a balanced-brace block beginning at `start_marker` inside
/// `src`. The returned slice includes the opening `{`, the matching
/// closing `}`, and everything in between (paren-depth-naive but
/// brace-balanced — sufficient for Rust match arms which always use
/// `{` for their bodies).
fn extract_brace_block_after<'a>(src: &'a str, start_marker: &str) -> &'a str {
    let arm_start = src
        .find(start_marker)
        .unwrap_or_else(|| panic!("source must contain `{start_marker}`"));
    let bytes = src.as_bytes();
    let mut i = arm_start + start_marker.len();
    // Skip whitespace until the opening `{`.
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    assert!(
        i < bytes.len() && bytes[i] == b'{',
        "expected `{{` after `{start_marker}`"
    );
    let body_start = i;
    let mut depth: i32 = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[body_start..=i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unterminated brace block starting at `{start_marker}`");
}

// ===========================================================================
// Scenario: BackgroundOutput translates StreamEvent::Tokens into session
//           updates and a StreamChunk::TokenUpdate
// ===========================================================================

#[test]
fn background_output_tokens_arm_updates_session_and_emits_token_update() {
    // @step Given the source of `rust/agent-loop/src/background_output.rs`
    let src = read_source("background_output.rs");

    // @step When I locate the `StreamEvent::Tokens(info)` arm of `BackgroundOutput::emit`
    let arm = extract_brace_block_after(&src, "StreamEvent::Tokens(info) =>");

    // @step Then the arm body calls `self.session.update_tokens(info.input_tokens as u32, info.output_tokens as u32)`
    assert!(
        arm.contains("self.session"),
        "Tokens arm must reach for `self.session.*`; arm was:\n{arm}"
    );
    assert!(
        arm.contains(".update_tokens(info.input_tokens as u32, info.output_tokens as u32)"),
        "Tokens arm must call \
         `.update_tokens(info.input_tokens as u32, info.output_tokens as u32)` on \
         `self.session`; arm was:\n{arm}"
    );

    // @step And the arm body calls `self.session.update_reasoning_tokens(r as u32)` inside an `if let Some(r) = info.reasoning_tokens` guard
    assert!(
        arm.contains("if let Some(r) = info.reasoning_tokens"),
        "Tokens arm must gate the reasoning-token update with \
         `if let Some(r) = info.reasoning_tokens`; arm was:\n{arm}"
    );
    assert!(
        arm.contains("self.session.update_reasoning_tokens(r as u32)"),
        "Tokens arm must call `self.session.update_reasoning_tokens(r as u32)` \
         inside the reasoning-token guard; arm was:\n{arm}"
    );
    let guard_pos = arm
        .find("if let Some(r) = info.reasoning_tokens")
        .expect("guard exists (checked above)");
    let call_pos = arm
        .find("self.session.update_reasoning_tokens(r as u32)")
        .expect("reasoning call exists (checked above)");
    assert!(
        guard_pos < call_pos,
        "`update_reasoning_tokens(r as u32)` must appear AFTER the \
         `if let Some(r) = info.reasoning_tokens` guard; arm was:\n{arm}"
    );

    // @step And the arm body constructs a `TokenTracker { ... }` literal populating all 8 fields
    assert!(
        arm.contains("TokenTracker {"),
        "Tokens arm must construct a `TokenTracker {{ ... }}` literal; arm was:\n{arm}"
    );
    for field in [
        "input_tokens: info.input_tokens as u32",
        "output_tokens: info.output_tokens as u32",
        "cache_read_input_tokens: info.cache_read_input_tokens.map(|v| v as u32)",
        "cache_creation_input_tokens: info.cache_creation_input_tokens.map(|v| v as u32)",
        "tokens_per_second: info.tokens_per_second",
        "cumulative_billed_input: None",
        "cumulative_billed_output: None",
        "reasoning_tokens: info.reasoning_tokens.map(|v| v as u32)",
    ] {
        assert!(
            arm.contains(field),
            "TokenTracker literal must initialise `{field}`; arm was:\n{arm}"
        );
    }

    // @step And the arm body returns the literal wrapped in `StreamChunk::token_update(...)`
    assert!(
        arm.contains("StreamChunk::token_update(TokenTracker {"),
        "Tokens arm must wrap the `TokenTracker {{ ... }}` literal in \
         `StreamChunk::token_update(...)`; arm was:\n{arm}"
    );
}

// ===========================================================================
// Scenario: BackgroundOutput translates StreamEvent::ContextFill into a
//           StreamChunk::ContextFillUpdate
// ===========================================================================

#[test]
fn background_output_context_fill_arm_emits_context_fill_update() {
    // @step Given the source of `rust/agent-loop/src/background_output.rs`
    let src = read_source("background_output.rs");

    // @step When I locate the `StreamEvent::ContextFill(info)` arm of `BackgroundOutput::emit`
    //
    // This arm is a single expression in the canonical body
    // (`=> StreamChunk::context_fill_update(ContextFillInfo { ... })`),
    // so we capture the substring up to the next `}),` terminator
    // rather than balancing braces.
    let arm_marker = "StreamEvent::ContextFill(info) =>";
    let arm_start = src
        .find(arm_marker)
        .unwrap_or_else(|| panic!("background_output.rs must contain `{arm_marker}`"));
    let arm_tail = &src[arm_start..];
    let terminator = arm_tail
        .find("}),")
        .expect("ContextFill arm must terminate with `}),`");
    let arm = &arm_tail[..=terminator + 1];

    // @step Then the arm body returns `StreamChunk::context_fill_update(ContextFillInfo { ... })`
    assert!(
        arm.contains("StreamChunk::context_fill_update(ContextFillInfo {"),
        "ContextFill arm must call \
         `StreamChunk::context_fill_update(ContextFillInfo {{ ... }})`; \
         arm was:\n{arm}"
    );

    // @step And the `ContextFillInfo` literal populates `fill_percentage`, `effective_tokens`, `threshold`, and `context_window`
    for field in [
        "fill_percentage: info.fill_percentage",
        "effective_tokens: info.effective_tokens as f64",
        "threshold: info.threshold as f64",
        "context_window: info.context_window as f64",
    ] {
        assert!(
            arm.contains(field),
            "ContextFillInfo literal must initialise `{field}`; arm was:\n{arm}"
        );
    }
}

// ===========================================================================
// Scenario: BackgroundSession exposes the cached token API expected by
//           BackgroundOutput
// ===========================================================================

#[cfg(feature = "test-support")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_session_round_trips_update_tokens_and_get_tokens() {
    use std::sync::Arc;

    use codelet_agent_loop::FspecAgentHooks;
    use codelet_sessions::session_manager::SessionManager;
    use uuid::Uuid;

    // Hermetic data dir + stub provider, mirroring rpc082_role_injection.
    let data_dir = tempfile::tempdir().expect("data dir tempdir");
    let _ = codelet_common::set_data_directory(data_dir.path().to_path_buf());

    let manager = Arc::new(SessionManager::new());
    manager.set_hooks(Arc::new(FspecAgentHooks::new()));

    codelet_providers::stub_provider::register_stub_provider();
    manager.set_default_model("stub/canned");

    let tmp = tempfile::tempdir().expect("tempdir");
    let project = tmp
        .path()
        .to_str()
        .expect("tempdir path is utf8")
        .to_string();
    let session_id_str = Uuid::new_v4().to_string();
    manager
        .create_session_with_id(
            &session_id_str,
            "stub/canned",
            &project,
            "token-test-session",
        )
        .await
        .expect("create_session_with_id");

    // @step Given a `codelet_sessions::background_session::BackgroundSession` constructed via test helpers
    let session = manager
        .get_session(&session_id_str)
        .expect("session must exist after create_session_with_id");

    // Sanity: a fresh session reports zero tokens. `get_tokens` collapses a
    // zero `cached_reasoning_tokens` AtomicU32 to `None` (see
    // rust/sessions/src/background_session.rs:717), so the expected
    // baseline is `(0, 0, None)`.
    let baseline = session.get_tokens();
    assert_eq!(
        baseline,
        (0, 0, None),
        "fresh BackgroundSession must report zero cached tokens; got {baseline:?}"
    );

    // @step When I call `session.update_tokens(100, 50)` and then `session.update_reasoning_tokens(25)`
    session.update_tokens(100, 50);
    session.update_reasoning_tokens(25);

    // @step Then `session.get_tokens()` returns `(100, 50, Some(25))`
    let tokens = session.get_tokens();
    assert_eq!(
        tokens,
        (100, 50, Some(25)),
        "session.get_tokens() must round-trip update_tokens + update_reasoning_tokens; \
         got {tokens:?}"
    );

    // @step And the underlying `cached_input_tokens`, `cached_output_tokens`, and `cached_reasoning_tokens` `AtomicU32` fields hold the same values
    use std::sync::atomic::Ordering;
    assert_eq!(
        session.cached_input_tokens.load(Ordering::Acquire),
        100,
        "cached_input_tokens AtomicU32 must hold the value written by update_tokens"
    );
    assert_eq!(
        session.cached_output_tokens.load(Ordering::Acquire),
        50,
        "cached_output_tokens AtomicU32 must hold the value written by update_tokens"
    );
    assert_eq!(
        session.cached_reasoning_tokens.load(Ordering::Acquire),
        25,
        "cached_reasoning_tokens AtomicU32 must hold the value written by update_reasoning_tokens"
    );
}

// ===========================================================================
// Scenario: codelet_rpc_types::StreamChunk declares TokenUpdate and
//           ContextFillUpdate with matching constructors
// ===========================================================================

#[test]
fn stream_chunk_declares_token_and_context_fill_variants_and_constructors() {
    // @step Given the source of `rust/rpc-types/src/lib.rs`
    let src = read_workspace_source("rust/rpc-types/src/lib.rs");

    // @step When I inspect the `StreamChunk` enum
    assert!(
        src.contains("pub enum StreamChunk"),
        "rust/rpc-types/src/lib.rs must define `pub enum StreamChunk`"
    );

    // @step Then the enum declares a `TokenUpdate { tokens: TokenTracker }` variant
    //
    // rpc-types/src/lib.rs is verbose-formatted with the variant fields
    // on their own lines; check the field line as a delimited substring
    // anchored after the variant marker.
    let token_variant_pos = src
        .find("    TokenUpdate {")
        .expect("StreamChunk must declare a `TokenUpdate {` variant on its own line");
    let token_variant_tail = &src[token_variant_pos..];
    let token_variant_end = token_variant_tail
        .find("    },")
        .expect("`TokenUpdate {` variant must terminate with `    },`");
    let token_variant = &token_variant_tail[..token_variant_end + 6];
    assert!(
        token_variant.contains("tokens: TokenTracker"),
        "StreamChunk::TokenUpdate variant must carry `tokens: TokenTracker`; \
         variant was:\n{token_variant}"
    );

    // @step And the enum declares a `ContextFillUpdate { context_fill: ContextFillInfo }` variant
    let ctx_variant_pos = src
        .find("    ContextFillUpdate {")
        .expect("StreamChunk must declare a `ContextFillUpdate {` variant on its own line");
    let ctx_variant_tail = &src[ctx_variant_pos..];
    let ctx_variant_end = ctx_variant_tail
        .find("    },")
        .expect("`ContextFillUpdate {` variant must terminate with `    },`");
    let ctx_variant = &ctx_variant_tail[..ctx_variant_end + 6];
    assert!(
        ctx_variant.contains("context_fill: ContextFillInfo"),
        "StreamChunk::ContextFillUpdate variant must carry \
         `context_fill: ContextFillInfo`; variant was:\n{ctx_variant}"
    );

    // @step And the impl block defines a `pub fn token_update(tokens: TokenTracker) -> Self` constructor returning `Self::TokenUpdate { tokens }`
    assert!(
        src.contains("pub fn token_update(tokens: TokenTracker) -> Self"),
        "rpc-types must expose `pub fn token_update(tokens: TokenTracker) -> Self`"
    );
    let token_ctor =
        extract_brace_block_after(&src, "pub fn token_update(tokens: TokenTracker) -> Self");
    assert!(
        token_ctor.contains("Self::TokenUpdate { tokens }"),
        "`token_update` ctor must return `Self::TokenUpdate {{ tokens }}`; body was:\n{token_ctor}"
    );

    // @step And the impl block defines a `pub fn context_fill_update(info: ContextFillInfo) -> Self` constructor returning `Self::ContextFillUpdate { context_fill: info }`
    assert!(
        src.contains("pub fn context_fill_update(info: ContextFillInfo) -> Self"),
        "rpc-types must expose `pub fn context_fill_update(info: ContextFillInfo) -> Self`"
    );
    let ctx_ctor = extract_brace_block_after(
        &src,
        "pub fn context_fill_update(info: ContextFillInfo) -> Self",
    );
    assert!(
        ctx_ctor.contains("Self::ContextFillUpdate { context_fill: info }"),
        "`context_fill_update` ctor must return \
         `Self::ContextFillUpdate {{ context_fill: info }}`; body was:\n{ctx_ctor}"
    );
}

// ===========================================================================
// Scenario: TokenInfo and ContextFillInfo source types carry the fields
//           BackgroundOutput consumes
// ===========================================================================

#[test]
fn token_info_and_context_fill_info_declare_expected_fields_and_stream_event_variants() {
    // @step Given the source of `rust/cli/src/interactive/output.rs`
    let src = read_workspace_source("rust/cli/src/interactive/output.rs");

    // @step When I inspect the `TokenInfo` and `ContextFillInfo` structs
    assert!(
        src.contains("pub struct TokenInfo {"),
        "output.rs must define `pub struct TokenInfo`"
    );
    let token_info_block = extract_brace_block_after(&src, "pub struct TokenInfo");
    assert!(
        src.contains("pub struct ContextFillInfo {"),
        "output.rs must define `pub struct ContextFillInfo`"
    );
    let ctx_info_block = extract_brace_block_after(&src, "pub struct ContextFillInfo");

    // @step Then `TokenInfo` declares `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens`, `tokens_per_second`, and `reasoning_tokens` fields
    for field in [
        "pub input_tokens: u64",
        "pub output_tokens: u64",
        "pub cache_read_input_tokens: Option<u64>",
        "pub cache_creation_input_tokens: Option<u64>",
        "pub tokens_per_second: Option<f64>",
        "pub reasoning_tokens: Option<u64>",
    ] {
        assert!(
            token_info_block.contains(field),
            "TokenInfo must declare `{field}`; struct was:\n{token_info_block}"
        );
    }

    // @step And `ContextFillInfo` declares `fill_percentage`, `effective_tokens`, `threshold`, and `context_window` fields
    for field in [
        "pub fill_percentage: u32",
        "pub effective_tokens: u64",
        "pub threshold: u64",
        "pub context_window: u64",
    ] {
        assert!(
            ctx_info_block.contains(field),
            "ContextFillInfo must declare `{field}`; struct was:\n{ctx_info_block}"
        );
    }

    // @step And the `StreamEvent` enum declares both a `Tokens(TokenInfo)` and a `ContextFill(ContextFillInfo)` variant
    assert!(
        src.contains("pub enum StreamEvent {"),
        "output.rs must define `pub enum StreamEvent`"
    );
    let stream_event_block = extract_brace_block_after(&src, "pub enum StreamEvent");
    assert!(
        stream_event_block.contains("Tokens(TokenInfo)"),
        "StreamEvent must declare `Tokens(TokenInfo)`; enum was:\n{stream_event_block}"
    );
    assert!(
        stream_event_block.contains("ContextFill(ContextFillInfo)"),
        "StreamEvent must declare `ContextFill(ContextFillInfo)`; enum was:\n{stream_event_block}"
    );
}
