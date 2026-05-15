//! RPC-018 — Source-shape regression for the AgentView chrome port +
//! shared types + new RPC methods.
//!
//! Feature: spec/features/rpc018-source-shape.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn src_dir() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn rpc_types_lib() -> std::path::PathBuf {
    common::workspace_root().join("rpc-types").join("src").join("lib.rs")
}

fn rpc_lib() -> std::path::PathBuf {
    common::workspace_root().join("rpc").join("src").join("lib.rs")
}

fn core_session_manager_handle() -> std::path::PathBuf {
    common::workspace_root().join("core").join("src").join("session_manager_handle.rs")
}

fn napi_git_rs() -> std::path::PathBuf {
    common::workspace_root().join("napi").join("src").join("git.rs")
}

fn napi_src_dir() -> std::path::PathBuf {
    common::workspace_root().join("napi").join("src")
}

fn project_root() -> std::path::PathBuf {
    common::workspace_root()
        .parent()
        .expect("project root above codelet/")
        .to_path_buf()
}

fn read_raw(path: &std::path::Path) -> String {
    common::read_to_string_or_panic(path)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    common::read_to_string_or_panic(path).lines().count()
}

/// Scenario: New shared types live in rpc-types
#[test]
fn new_shared_types_live_in_rpc_types() {
    // @step Given codelet/rpc-types/src/lib.rs after RPC-018 lands
    let body = read_raw(&rpc_types_lib());
    // @step Then the file contains the substring "pub struct ModelInfo"
    assert!(body.contains("pub struct ModelInfo"), "rpc-types must define `pub struct ModelInfo`");
    // @step And the file contains the substring "pub display_name: String"
    assert!(body.contains("pub display_name: String"), "ModelInfo must have `pub display_name: String`");
    // @step And the file contains the substring "pub supports_reasoning: bool"
    assert!(body.contains("pub supports_reasoning: bool"), "ModelInfo must have `pub supports_reasoning: bool`");
    // @step And the file contains the substring "pub supports_vision: bool"
    assert!(body.contains("pub supports_vision: bool"), "ModelInfo must have `pub supports_vision: bool`");
    // @step And the file contains the substring "pub context_window: u32"
    assert!(body.contains("pub context_window: u32"), "ModelInfo must have `pub context_window: u32`");
    // @step And the file contains the substring "pub enum ThinkingLevel"
    assert!(body.contains("pub enum ThinkingLevel"), "rpc-types must define `pub enum ThinkingLevel`");
    // @step And the file contains the substring "pub struct WorkspaceInfo"
    assert!(body.contains("pub struct WorkspaceInfo"), "rpc-types must define `pub struct WorkspaceInfo`");
    // @step And the file contains the substring "pub cwd: String"
    assert!(body.contains("pub cwd: String"), "WorkspaceInfo must have `pub cwd: String`");
    // @step And the file contains the substring "pub git_branch: Option<String>"
    assert!(body.contains("pub git_branch: Option<String>"), "WorkspaceInfo must have `pub git_branch: Option<String>`");
}

/// Scenario: FspecService trait gains three new RPC methods
#[test]
fn fspec_service_trait_gains_three_new_rpc_methods() {
    // @step Given codelet/rpc/src/lib.rs after RPC-018 lands
    let body = read_raw(&rpc_lib());
    // @step Then the file contains the substring "async fn get_model_info(session_id: SessionId) -> ModelInfo"
    assert!(
        body.contains("async fn get_model_info(session_id: SessionId) -> ModelInfo"),
        "FspecService must declare `async fn get_model_info(session_id: SessionId) -> ModelInfo`"
    );
    // @step And the file contains the substring "async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel"
    assert!(
        body.contains("async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel"),
        "FspecService must declare `async fn get_thinking_level(session_id: SessionId) -> ThinkingLevel`"
    );
    // @step And the file contains the substring "async fn get_workspace_info() -> WorkspaceInfo"
    assert!(
        body.contains("async fn get_workspace_info() -> WorkspaceInfo"),
        "FspecService must declare `async fn get_workspace_info() -> WorkspaceInfo`"
    );
    // @step And the FspecServiceImpl body contains the substring "codelet_git::status::get_current_branch"
    assert!(
        body.contains("codelet_git::status::get_current_branch"),
        "FspecServiceImpl::get_workspace_info must call codelet_git::status::get_current_branch"
    );
}

/// Scenario: SessionManagerHandle trait gains get_model_info / get_thinking_level with default impls
#[test]
fn session_manager_handle_trait_gains_two_new_methods_with_defaults() {
    // @step Given codelet/core/src/session_manager_handle.rs after RPC-018 lands
    let body = read_raw(&core_session_manager_handle());
    // @step Then the file contains the substring "fn get_model_info(&self, session_id: &SessionId) -> ModelInfo"
    assert!(
        body.contains("fn get_model_info(&self, session_id: &SessionId) -> ModelInfo"),
        "SessionManagerHandle must declare `fn get_model_info(&self, session_id: &SessionId) -> ModelInfo`"
    );
    // @step And the file contains the substring "fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel"
    assert!(
        body.contains("fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel"),
        "SessionManagerHandle must declare `fn get_thinking_level(&self, session_id: &SessionId) -> ThinkingLevel`"
    );
    // @step And both methods have default implementations returning the safe defaults
    assert!(
        body.contains("ModelInfo::default()"),
        "SessionManagerHandle default impl for get_model_info must return ModelInfo::default()"
    );
    assert!(
        body.contains("ThinkingLevel::Off"),
        "SessionManagerHandle default impl for get_thinking_level must return ThinkingLevel::Off"
    );
}

/// Scenario: FspecBackend trait declares the three new methods
#[test]
fn fspec_backend_trait_declares_the_three_new_methods() {
    // @step Given codelet/fspec-tui/src/transport/mod.rs after RPC-018 lands
    let body = read_raw(&src_dir().join("transport").join("mod.rs"));
    // @step Then the file contains the substring "async fn get_model_info"
    assert!(body.contains("async fn get_model_info"));
    // @step And the file contains the substring "async fn get_thinking_level"
    assert!(body.contains("async fn get_thinking_level"));
    // @step And the file contains the substring "async fn get_workspace_info"
    assert!(body.contains("async fn get_workspace_info"));
}

/// Scenario: Both transports implement the three new FspecBackend methods
#[test]
fn both_transports_implement_the_three_new_methods() {
    // @step Given the codelet/fspec-tui crate after RPC-018 lands
    let embedded = read_raw(&src_dir().join("transport").join("embedded.rs"));
    let websocket = read_raw(&src_dir().join("transport").join("websocket.rs"));
    // @step Then codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_model_info"
    assert!(embedded.contains("async fn get_model_info"));
    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_thinking_level"
    assert!(embedded.contains("async fn get_thinking_level"));
    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_workspace_info"
    assert!(embedded.contains("async fn get_workspace_info"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_model_info"
    assert!(websocket.contains("async fn get_model_info"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_thinking_level"
    assert!(websocket.contains("async fn get_thinking_level"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_workspace_info"
    assert!(websocket.contains("async fn get_workspace_info"));
}

/// Scenario: New agent widget modules exist as separate files
#[test]
fn new_agent_widget_modules_exist_as_separate_files() {
    // @step Given the codelet/fspec-tui crate after RPC-018 lands
    let header = src_dir().join("views").join("agent").join("header.rs");
    let footer = src_dir().join("views").join("agent").join("footer.rs");
    // @step Then the file codelet/fspec-tui/src/views/agent/header.rs exists
    assert!(header.exists(), "views/agent/header.rs must exist after RPC-018");
    // @step And the file codelet/fspec-tui/src/views/agent/footer.rs exists
    assert!(footer.exists(), "views/agent/footer.rs must exist after RPC-018");
}

/// Scenario: New and modified agent modules stay under 300 lines
#[test]
fn new_and_modified_agent_modules_stay_under_300_lines() {
    // @step Given the directory codelet/fspec-tui/src/views/agent/ plus the views/agent.rs orchestrator (or views/agent/mod.rs)
    let agent_dir = src_dir().join("views").join("agent");
    let agent_orchestrator_rs = src_dir().join("views").join("agent.rs");
    let agent_mod_rs = agent_dir.join("mod.rs");

    // @step When a test counts the line-count of every .rs file
    let mut targets: Vec<std::path::PathBuf> = Vec::new();
    if agent_orchestrator_rs.exists() {
        targets.push(agent_orchestrator_rs.clone());
    }
    if agent_dir.exists() {
        let entries = std::fs::read_dir(&agent_dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", agent_dir.display()));
        for entry in entries {
            let entry = entry.expect("read_dir entry");
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                targets.push(path);
            }
        }
    }
    // At least the orchestrator must exist somewhere.
    assert!(
        agent_orchestrator_rs.exists() || agent_mod_rs.exists(),
        "either views/agent.rs or views/agent/mod.rs must exist"
    );

    let mut violations = Vec::new();
    for path in &targets {
        let lines = count_lines_path(path);
        // @step Then every file in views/agent/ has fewer than 300 lines
        // @step And the orchestrator file (views/agent.rs OR views/agent/mod.rs) has fewer than 300 lines
        if lines >= 300 {
            violations.push(format!("{}: {} lines >= 300 ceiling", path.display(), lines));
        }
    }
    assert!(
        violations.is_empty(),
        "RPC-018 agent modules MUST stay < 300 LoC. Violations: {violations:?}"
    );
}

/// Scenario: Action enum gains three new variants
#[test]
fn action_enum_gains_three_new_variants() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-018 lands
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
    // @step Given codelet/napi/src/git.rs after RPC-018 lands
    let body = read_raw(&napi_git_rs());
    // @step Then the file contains the substring "pub fn get_workspace_info"
    assert!(
        body.contains("pub fn get_workspace_info"),
        "codelet/napi/src/git.rs must export `pub fn get_workspace_info`"
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
    // @step Given codelet/napi/src/session_manager.rs (or a sibling file) after RPC-018 lands
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
    // @step Then the codelet/napi/src tree contains the substring "pub fn get_model_info"
    assert!(
        found,
        "codelet/napi/src tree must include `pub fn get_model_info` somewhere"
    );
}

/// Scenario: Existing TS AgentView chrome files are untouched
#[test]
fn existing_ts_agentview_chrome_files_still_exist() {
    // @step Given the project root after RPC-018 lands
    let root = project_root();
    // @step Then the file src/tui/components/SessionHeader.tsx exists
    assert!(root.join("src/tui/components/SessionHeader.tsx").exists());
    // @step And the file src/tui/components/SessionFooter.tsx exists
    assert!(root.join("src/tui/components/SessionFooter.tsx").exists());
    // @step And the file src/tui/utils/tokenStateUtils.ts exists
    assert!(root.join("src/tui/utils/tokenStateUtils.ts").exists());
    // @step And the file src/tui/store/modelStore.ts exists
    assert!(root.join("src/tui/store/modelStore.ts").exists());
}

/// Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
#[test]
fn views_still_avoid_encapsulated_transport_crates_and_runtime_construction() {
    // @step Given the directory codelet/fspec-tui/src/views/ (including views/agent/) after RPC-018 lands
    let views_dir = src_dir().join("views");
    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        // @step Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
        for needle in ["codelet_napi::", "codelet_core::", "tarpc::", "tokio_tungstenite::"] {
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
