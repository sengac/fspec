/// Scenario: Action enum gains three new variants
#[test]
fn action_enum_gains_three_new_variants() {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-018 lands
    let body = read_raw(&src_dir().join("components").join("mod.rs"));
    // @step Then the file contains the substring "ModelInfoLoaded"
    assert!(body.contains("ModelInfoLoaded"));
    // @step And the file contains the substring "ThinkingLevelLoaded"
    assert!(body.contains("ThinkingLevelLoaded"));
    // @step And the file contains the substring "WorkspaceInfoLoaded"
    assert!(body.contains("WorkspaceInfoLoaded"));
}

/// Scenario: NAPI surface exposes additive get_workspace_info export
#[test]
fn napi_surface_exposes_additive_get_workspace_info_export() {
    // @step Given rust/napi/src/git.rs after RPC-018 lands
    let body = read_raw(&napi_git_rs());
    // @step Then the file contains the substring "pub fn get_workspace_info"
    assert!(
        body.contains("pub fn get_workspace_info"),
        "rust/napi/src/git.rs must export `pub fn get_workspace_info`"
    );
    // @step And the file contains the substring "codelet_git::status::get_current_branch"
    assert!(
        body.contains("codelet_git::status::get_current_branch"),
        "napi get_workspace_info must delegate to codelet_git::status::get_current_branch"
    );
}

/// Scenario: NAPI surface exposes additive get_model_info export
#[test]
fn napi_surface_exposes_additive_get_model_info_export() {
    // @step Given rust/napi/src/session_manager.rs (or a sibling file) after RPC-018 lands
    let dir = napi_src_dir();
    let entries = std::fs::read_dir(&dir).expect("read napi/src");
    let mut found = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let body = std::fs::read_to_string(&path).unwrap_or_default();
        if body.contains("pub fn get_model_info") {
            found = true;
            break;
        }
    }
    // @step Then the rust/napi/src tree contains the substring "pub fn get_model_info"
    assert!(
        found,
        "rust/napi/src tree must include `pub fn get_model_info` somewhere"
    );
}

/// Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
#[test]
fn views_still_avoid_encapsulated_transport_crates_and_runtime_construction() {
    // @step Given the directory rust/fspec-tui/src/views/ (including views/agent/) after RPC-018 lands
    let views_dir = src_dir().join("views");
    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        // @step Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
        for needle in [
            "codelet_napi::",
            "codelet_core::",
            "tarpc::",
            "tokio_tungstenite::",
        ] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
        // @step And no file constructs `tokio::runtime::Builder` or `Runtime::new()`
        for needle in ["tokio::runtime::Builder", "Runtime::new()"] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "RPC-018 must preserve transport-encapsulation + host-runtime invariants. Violations: {violations:?}"
    );
}
