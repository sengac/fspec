//! RPC-062: Source-shape regression tests pinning the MCP injection
//! wiring inside `codelet/sessions/src/session_manager.rs` and the
//! NAPI-side consumer in `codelet/napi/src/agent_loop.rs`, plus a
//! negative-grep assertion that no MCP method has leaked into the RPC
//! surface.
//!
//! Feature: spec/features/rpc-062-mcp-injection-source-shape.feature
//!
//! Pattern mirrors the source-shape helpers from
//! `codelet/sessions/tests/handle_impl.rs` (RPC-042) and
//! `codelet/sessions/tests/no_napi_dependency.rs` (RPC-044).

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
    // @step Given the file codelet/sessions/src/session_manager.rs is compiled
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
// Scenario: session_manager.rs calls init_mcp_session in both create paths
// =============================================================================
#[test]
fn scenario_session_manager_calls_init_mcp_in_both_create_paths() {
    // @step Given the file codelet/sessions/src/session_manager.rs is compiled
    let raw = session_manager_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then it contains exactly two occurrences of the substring "codelet_tools::init_mcp_session(uuid)"
    let init_count = src.matches("codelet_tools::init_mcp_session(uuid)").count();
    assert_eq!(
        init_count, 2,
        "expected exactly two `codelet_tools::init_mcp_session(uuid)` call sites in session_manager.rs, found {init_count}",
    );

    // @step And one occurrence sits inside the body of "pub async fn create_session_with_id"
    let create_body = extract_fn_body(&src, "pub async fn create_session_with_id");
    let init_in_create = create_body
        .matches("codelet_tools::init_mcp_session(uuid)")
        .count();
    assert_eq!(
        init_in_create, 1,
        "expected exactly one `codelet_tools::init_mcp_session(uuid)` inside `create_session_with_id`, found {init_in_create}",
    );

    // @step And the other occurrence sits inside the body of "pub async fn create_isolated_session_with_id"
    let isolated_body = extract_fn_body(&src, "pub async fn create_isolated_session_with_id");
    let init_in_isolated = isolated_body
        .matches("codelet_tools::init_mcp_session(uuid)")
        .count();
    assert_eq!(
        init_in_isolated, 1,
        "expected exactly one `codelet_tools::init_mcp_session(uuid)` inside `create_isolated_session_with_id`, found {init_in_isolated}",
    );

    // @step And each occurrence is followed by an invocation of "spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)"
    let spawn_call = "spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)";
    assert!(
        create_body.contains(spawn_call),
        "create_session_with_id body must follow init_mcp_session with `{spawn_call}`",
    );
    assert!(
        isolated_body.contains(spawn_call),
        "create_isolated_session_with_id body must follow init_mcp_session with `{spawn_call}`",
    );
}

// =============================================================================
// Scenario: session_manager.rs calls cleanup_mcp_session in destroy_session
// =============================================================================
#[test]
fn scenario_session_manager_calls_cleanup_mcp_in_destroy_session() {
    // @step Given the file codelet/sessions/src/session_manager.rs is compiled
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
    // @step Given the file codelet/sessions/src/session_manager.rs is compiled
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

// =============================================================================
// Scenario: codelet-napi agent_loop consumes mcp_injection_rx inside
//           its select! loop
// =============================================================================
#[test]
fn scenario_napi_agent_loop_consumes_mcp_injection_rx() {
    // @step Given the file codelet/napi/src/agent_loop.rs is compiled
    let raw = agent_loop_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then the file declares a function whose signature contains "mut mcp_injection_rx: mpsc::Receiver<McpInjection>"
    let sig_count = src
        .matches("mut mcp_injection_rx: mpsc::Receiver<McpInjection>")
        .count();
    assert!(
        sig_count >= 1,
        "codelet/napi/src/agent_loop.rs must declare a function taking `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` (found {sig_count} matches)",
    );

    // @step And the file contains at least one occurrence of "mcp_injection_rx.recv()" inside the agent loop body
    let recv_count = src.matches("mcp_injection_rx.recv()").count();
    assert!(
        recv_count >= 1,
        "codelet/napi/src/agent_loop.rs must contain at least one `mcp_injection_rx.recv()` arm (found {recv_count})",
    );
}

// =============================================================================
// Scenario: No MCP injection methods leak into the RPC surface across
//           handle, service, and backend traits
// =============================================================================
#[test]
fn scenario_no_mcp_method_leaks_into_rpc_surface() {
    // @step Given the files codelet/core/src/session_manager_handle.rs, codelet/rpc/src/lib.rs, and codelet/fspec-tui/src/transport/mod.rs are compiled
    let targets = [
        (
            "codelet/core/src/session_manager_handle.rs",
            workspace_root()
                .join("core")
                .join("src")
                .join("session_manager_handle.rs"),
        ),
        (
            "codelet/rpc/src/lib.rs",
            workspace_root().join("rpc").join("src").join("lib.rs"),
        ),
        (
            "codelet/fspec-tui/src/transport/mod.rs",
            workspace_root()
                .join("fspec-tui")
                .join("src")
                .join("transport")
                .join("mod.rs"),
        ),
    ];

    // @step When I scan their source bytes after stripping Rust comments
    let scanned: Vec<(&'static str, String)> = targets
        .iter()
        .map(|(label, path)| (*label, strip_rust_comments(&read(path))))
        .collect();

    // @step Then no file contains the substring "init_mcp"
    for (label, src) in &scanned {
        assert!(
            !src.contains("init_mcp"),
            "{label} MUST NOT mention `init_mcp` — MCP injection stays internal to the agent loop",
        );
    }

    // @step And no file contains the substring "cleanup_mcp"
    for (label, src) in &scanned {
        assert!(
            !src.contains("cleanup_mcp"),
            "{label} MUST NOT mention `cleanup_mcp` — MCP injection stays internal to the agent loop",
        );
    }

    // @step And no file contains the substring "mcp_session"
    for (label, src) in &scanned {
        assert!(
            !src.contains("mcp_session"),
            "{label} MUST NOT mention `mcp_session` — MCP injection stays internal to the agent loop",
        );
    }

    // @step And no file contains the substring "mcp_injection"
    for (label, src) in &scanned {
        assert!(
            !src.contains("mcp_injection"),
            "{label} MUST NOT mention `mcp_injection` — MCP injection stays internal to the agent loop",
        );
    }
}

// =============================================================================
// Scenario: codelet-sessions has no transitive napi dependency after
//           the RPC-062 audit
// =============================================================================
#[test]
fn scenario_codelet_sessions_has_no_transitive_napi_dependency() {
    // @step Given the existing test codelet/sessions/tests/no_napi_dependency.rs is in the codelet-sessions test suite
    let no_napi_dep = workspace_root()
        .join("sessions")
        .join("tests")
        .join("no_napi_dependency.rs");
    assert!(
        no_napi_dep.is_file(),
        "RPC-044's no_napi_dependency.rs must still exist in codelet-sessions tests",
    );

    // @step When I run cargo test -p codelet-sessions --test no_napi_dependency
    //
    // We avoid recursively invoking `cargo test` (slow + can deadlock
    // against the parent runner). Instead we re-execute the same
    // `cargo metadata` walk that the companion test performs, so a
    // regression here would fail BOTH this test AND the companion.
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");

    // @step Then both scenarios in that test file pass without modification
    assert!(
        output.status.success(),
        "cargo metadata must succeed; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("metadata JSON must parse");
    let packages = json
        .get("packages")
        .and_then(|v| v.as_array())
        .expect("packages must be an array");
    let resolve = json.get("resolve").expect("resolve must exist");
    let nodes = resolve
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("resolve.nodes must be an array");
    let root_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("codelet-sessions"))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .expect("codelet-sessions package must exist in metadata")
        .to_string();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stack: Vec<String> = vec![root_id];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|i| i.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for d in deps {
                        if let Some(s) = d.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
                break;
            }
        }
    }
    let mut transitive_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for id in &seen {
        if let Some(pkg) = packages
            .iter()
            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                transitive_names.insert(name.to_string());
            }
        }
    }

    // @step And the codelet-sessions transitive dependency graph contains zero occurrences of the codelet-napi package
    assert!(
        !transitive_names.contains("codelet-napi"),
        "codelet-sessions MUST NOT transitively depend on codelet-napi (RPC-044 forbidden arrow). Transitive set: {transitive_names:?}",
    );
}
