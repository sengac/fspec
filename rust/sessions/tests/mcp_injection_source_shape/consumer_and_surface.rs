// =============================================================================
// Scenario: codelet-napi agent_loop consumes mcp_injection_rx inside
//           its select! loop
// =============================================================================
#[test]
fn scenario_napi_agent_loop_consumes_mcp_injection_rx() {
    // @step Given the file rust/napi/src/agent_loop.rs is compiled
    let raw = agent_loop_src();

    // @step When I scan its source bytes after stripping Rust comments
    let src = strip_rust_comments(&raw);

    // @step Then the file declares a function whose signature contains "mut mcp_injection_rx: mpsc::Receiver<McpInjection>"
    let sig_count = src
        .matches("mut mcp_injection_rx: mpsc::Receiver<McpInjection>")
        .count();
    assert!(
        sig_count >= 1,
        "rust/napi/src/agent_loop.rs must declare a function taking `mut mcp_injection_rx: mpsc::Receiver<McpInjection>` (found {sig_count} matches)",
    );

    // @step And the file contains at least one occurrence of "mcp_injection_rx.recv()" inside the agent loop body
    let recv_count = src.matches("mcp_injection_rx.recv()").count();
    assert!(
        recv_count >= 1,
        "rust/napi/src/agent_loop.rs must contain at least one `mcp_injection_rx.recv()` arm (found {recv_count})",
    );
}

// =============================================================================
// Scenario: No MCP injection methods leak into the RPC surface across
//           handle, service, and backend traits
// =============================================================================
#[test]
fn scenario_no_mcp_method_leaks_into_rpc_surface() {
    // @step Given the files rust/core/src/session_manager_handle.rs, rust/rpc/src/lib.rs, and rust/fspec-tui/src/transport/mod.rs are compiled
    let targets = [
        (
            "rust/core/src/session_manager_handle.rs",
            workspace_root()
                .join("core")
                .join("src")
                .join("session_manager_handle.rs"),
        ),
        (
            "rust/rpc/src/lib.rs",
            workspace_root().join("rpc").join("src").join("lib.rs"),
        ),
        (
            "rust/fspec-tui/src/transport/mod.rs",
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
        .map(|(label, path)| {
            let source = codelet_test_helpers::source_composition::read_source(path)
                .expect("resolve RPC surface source");
            (*label, strip_rust_comments(&source))
        })
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
    // @step Given the existing test rust/sessions/tests/no_napi_dependency.rs is in the codelet-sessions test suite
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
