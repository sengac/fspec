//! RPC-062: Source-shape regression tests pinning the MCP injection
//! wiring inside `rust/sessions/src/session_manager.rs` and the
//! NAPI-side consumer in `rust/napi/src/agent_loop.rs`, plus a
//! negative-grep assertion that no MCP method has leaked into the RPC
//! surface.
//!
//! Feature: spec/features/rpc-062-mcp-injection-source-shape.feature
//!
//! Pattern mirrors the source-shape helpers from
//! `rust/sessions/tests/handle_impl.rs` (RPC-042) and
//! `rust/sessions/tests/no_napi_dependency.rs` (RPC-044).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

// =============================================================================
// Path / read helpers — sibling of handle_impl.rs's helper set
// =============================================================================

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Strip both `//` line comments and `/* ... */` block comments from
/// Rust source so that substring scans don't get fooled by needle
/// references inside doc comments.
fn strip_rust_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

/// Walk the comment-stripped source and return the substring between
/// the byte where `header` first appears and the matching closing `}`
/// of that function body (best-effort brace counter).
fn extract_fn_body<'a>(src: &'a str, header: &str) -> &'a str {
    let start = src
        .find(header)
        .unwrap_or_else(|| panic!("expected to find function header `{header}` in source"));
    let body_start_rel = src[start..]
        .find('{')
        .unwrap_or_else(|| panic!("expected an opening `{{` after `{header}`"));
    let body_start = start + body_start_rel + 1;
    let bytes = src.as_bytes();
    let mut depth = 1usize;
    let mut i = body_start;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[body_start..i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    panic!("function body for `{header}` not terminated by a matching `}}`")
}

fn session_manager_src() -> String {
    let p = workspace_root()
        .join("sessions")
        .join("src")
        .join("session_manager.rs");
    read(&p)
}

fn agent_loop_src() -> String {
    let p = workspace_root()
        .join("napi")
        .join("src")
        .join("agent_loop.rs");
    read(&p)
}

// =============================================================================
// Scenario: codelet-sessions imports the NAPI-free McpInjection type
//           from codelet-tools
// =============================================================================
#[test]
fn scenario_codelet_sessions_imports_napi_free_mcp_injection() {
    // @step Given the file rust/sessions/src/session_manager.rs is compiled
    let raw = session_manager_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then it contains exactly one occurrence of the substring "use codelet_tools::McpInjection;"
    let import_count = src.matches("use codelet_tools::McpInjection;").count();
    assert_eq!(
        import_count, 1,
        "expected exactly one `use codelet_tools::McpInjection;` in session_manager.rs, found {import_count}",
    );

    // @step And it contains zero local definitions of "enum McpInjection" or "struct McpInjection"
    let enum_def = src.matches("enum McpInjection").count();
    let struct_def = src.matches("struct McpInjection").count();
    assert_eq!(
        enum_def, 0,
        "session_manager.rs MUST NOT redefine McpInjection (found {enum_def} `enum McpInjection` matches)",
    );
    assert_eq!(
        struct_def, 0,
        "session_manager.rs MUST NOT redefine McpInjection (found {struct_def} `struct McpInjection` matches)",
    );
}

// =============================================================================
// Scenario: session_manager.rs delegates MCP init to the shared helper in
// both create paths (WT-012: the isolated path no longer inlines it)
// =============================================================================
#[test]
fn scenario_session_manager_calls_init_mcp_in_both_create_paths() {
    // @step Given the file rust/sessions/src/session_manager.rs is compiled
    let raw = session_manager_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then the isolated create path delegates MCP init to the shared helper
    // (WT-012: `create_isolated_session_with_id` routes through
    // `create_background_session_inner`, whose body in session_creation_helper.rs
    // carries the `codelet_tools::init_mcp_session(uuid)` call — zero inline
    // occurrences may remain in session_manager.rs)
    let helper_src = strip_rust_comments(&read(
        &workspace_root().join("sessions/src/session_creation_helper.rs"),
    ));
    let helper_init = helper_src
        .matches("codelet_tools::init_mcp_session(uuid)")
        .count();
    assert_eq!(
        helper_init, 1,
        "expected exactly one `codelet_tools::init_mcp_session(uuid)` call site in \
         session_creation_helper.rs (the shared create path), found {helper_init}"
    );

    // @step And the shared helper is the single MCP init site for both create paths
    // (zero inline `codelet_tools::init_mcp_session(uuid)` occurrences in session_manager.rs)
    let sm_inline = src.matches("codelet_tools::init_mcp_session(uuid)").count();
    assert_eq!(
        sm_inline, 0,
        "WT-012: both create paths must source `init_mcp_session` from the shared \
         helper — session_manager.rs must carry zero inline occurrences (found {sm_inline})"
    );

    // @step And each create path passes the helper's mcp_injection_rx to
    // "spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)"
    let spawn_call = "spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)";
    let create_body = extract_fn_body(&src, "pub async fn create_session_with_id");
    assert!(
        create_body.contains(spawn_call),
        "create_session_with_id body must pass the helper's mcp_injection_rx via `{spawn_call}`"
    );
    let isolated_body = extract_fn_body(&src, "pub async fn create_isolated_session_with_id");
    assert!(
        isolated_body.contains("create_background_session_inner"),
        "create_isolated_session_with_id must delegate to create_background_session_inner"
    );
    assert!(
        isolated_body.contains(spawn_call),
        "create_isolated_session_with_id body must pass the helper's mcp_injection_rx via `{spawn_call}`"
    );
}

// =============================================================================
// Scenario: session_manager.rs calls cleanup_mcp_session in destroy_session
// =============================================================================
#[test]
fn scenario_session_manager_calls_cleanup_mcp_in_destroy_session() {
    // @step Given the file rust/sessions/src/session_manager.rs is compiled
    let raw = session_manager_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then it contains exactly one occurrence of the substring "codelet_tools::cleanup_mcp_session(uuid)"
    let cleanup_count = src
        .matches("codelet_tools::cleanup_mcp_session(uuid)")
        .count();
    assert_eq!(
        cleanup_count, 1,
        "expected exactly one `codelet_tools::cleanup_mcp_session(uuid)` call site in session_manager.rs, found {cleanup_count}",
    );

    // @step And the occurrence sits inside the body of "pub fn destroy_session"
    let destroy_body = extract_fn_body(&src, "pub fn destroy_session");
    let cleanup_in_destroy = destroy_body
        .matches("codelet_tools::cleanup_mcp_session(uuid)")
        .count();
    assert_eq!(
        cleanup_in_destroy, 1,
        "expected exactly one `codelet_tools::cleanup_mcp_session(uuid)` inside `destroy_session`, found {cleanup_in_destroy}",
    );
}

// =============================================================================
// Scenario: SessionManagerHooks trait declares spawn_agent_loop with
//           the mcp_injection_rx parameter
// =============================================================================
#[test]
fn scenario_session_manager_hooks_declares_spawn_agent_loop_with_mcp_injection_rx() {
    // @step Given the file rust/sessions/src/session_manager.rs is compiled
    let raw = session_manager_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then the SessionManagerHooks trait declares a method named spawn_agent_loop
    let trait_header = "pub trait SessionManagerHooks";
    let trait_idx = src
        .find(trait_header)
        .unwrap_or_else(|| panic!("expected `{trait_header}` in session_manager.rs"));
    // Constrain the search to the body that immediately follows the
    // trait header (we deliberately do not require an exact closing
    // brace match — the next `pub trait`/`impl`/`pub struct` line is a
    // safe upper bound).
    let after_trait = &src[trait_idx..];
    let end = after_trait
        .find("\nimpl ")
        .or_else(|| after_trait.find("\npub struct "))
        .or_else(|| after_trait.find("\npub trait "))
        .unwrap_or(after_trait.len());
    let trait_body = &after_trait[..end];

    assert!(
        trait_body.contains("fn spawn_agent_loop"),
        "SessionManagerHooks must declare `fn spawn_agent_loop` (searched range: {} chars)",
        trait_body.len()
    );

    // @step And the spawn_agent_loop signature contains the parameter "mcp_injection_rx: mpsc::Receiver<McpInjection>"
    assert!(
        trait_body.contains("mcp_injection_rx: mpsc::Receiver<McpInjection>"),
        "SessionManagerHooks::spawn_agent_loop must accept `mcp_injection_rx: mpsc::Receiver<McpInjection>`",
    );
}

include!("mcp_injection_source_shape/consumer_and_surface.rs");
