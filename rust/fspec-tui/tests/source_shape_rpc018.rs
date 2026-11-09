//! RPC-018 — Source-shape regression for the AgentView chrome port +
//! shared types + new RPC methods.
//!
//! Feature: spec/features/rpc018-source-shape.feature

mod common;
mod source_shape_support;
use codelet_test_helpers::{OrFail, TestResult};
use source_shape_support::*;

/// Scenario: New shared types live in rpc-types
#[test]
fn new_shared_types_live_in_rpc_types() -> TestResult {
    // @step Given rust/rpc-types/src/lib.rs after RPC-018 lands
    let body = read_tree(&rpc_types_lib()?)?;
    // @step Then the file contains the substring "pub struct ModelInfo"
    assert!(
        body.contains("pub struct ModelInfo"),
        "rpc-types must define `pub struct ModelInfo`"
    );
    // @step And the file contains the substring "pub display_name: String"
    assert!(
        body.contains("pub display_name: String"),
        "ModelInfo must have `pub display_name: String`"
    );
    // @step And the file contains the substring "pub supports_reasoning: bool"
    assert!(
        body.contains("pub supports_reasoning: bool"),
        "ModelInfo must have `pub supports_reasoning: bool`"
    );
    // @step And the file contains the substring "pub supports_vision: bool"
    assert!(
        body.contains("pub supports_vision: bool"),
        "ModelInfo must have `pub supports_vision: bool`"
    );
    // @step And the file contains the substring "pub context_window: Option<u32>"
    assert!(
        body.contains("pub context_window: Option<u32>"),
        "ModelInfo must have `pub context_window: Option<u32>`"
    );
    // @step And the file contains the substring "pub enum ThinkingLevel"
    assert!(
        body.contains("pub enum ThinkingLevel"),
        "rpc-types must define `pub enum ThinkingLevel`"
    );
    // @step And the file contains the substring "pub struct WorkspaceInfo"
    assert!(
        body.contains("pub struct WorkspaceInfo"),
        "rpc-types must define `pub struct WorkspaceInfo`"
    );
    // @step And the file contains the substring "pub cwd: String"
    assert!(
        body.contains("pub cwd: String"),
        "WorkspaceInfo must have `pub cwd: String`"
    );
    // @step And the file contains the substring "pub git_branch: Option<String>"
    assert!(
        body.contains("pub git_branch: Option<String>"),
        "WorkspaceInfo must have `pub git_branch: Option<String>`"
    );
    Ok(())
}

/// Scenario: FspecService trait gains three new RPC methods
#[test]
fn fspec_service_trait_gains_three_new_rpc_methods() -> TestResult {
    // @step Given rust/rpc/src/lib.rs after RPC-018 lands
    let body = read_raw(&rpc_lib()?)?;
    // @step Then the file contains the substring "async fn get_model_info(session_id: SessionId) -> Result<ModelInfo, String>"
    assert!(
        body.contains("async fn get_model_info(session_id: SessionId) -> Result<ModelInfo, String>"),
        "FspecService must declare `async fn get_model_info(session_id: SessionId) -> Result<ModelInfo, String>`"
    );
    // @step And the file contains the substring "async fn get_thinking_level(session_id: SessionId) -> Result<ThinkingLevel, String>"
    assert!(
        body.contains("async fn get_thinking_level(session_id: SessionId) -> Result<ThinkingLevel, String>"),
        "FspecService must declare `async fn get_thinking_level(session_id: SessionId) -> Result<ThinkingLevel, String>`"
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
    Ok(())
}

/// Scenario: SessionManagerHandle trait gains get_model_info / get_thinking_level with default impls
#[test]
fn session_manager_handle_trait_gains_two_new_methods_with_defaults() -> TestResult {
    // @step Given rust/core/src/session_manager_handle.rs after RPC-018 lands
    let body = read_tree(&core_session_manager_handle()?)?;
    // @step Then the file contains the substring "fn get_model_info(&self, session_id: &SessionId) -> Result<ModelInfo, String>"
    assert!(
        body.contains("fn get_model_info(&self, session_id: &SessionId) -> Result<ModelInfo, String>"),
        "SessionManagerHandle must declare `fn get_model_info(&self, session_id: &SessionId) -> Result<ModelInfo, String>`"
    );
    // @step And the file contains the substring "fn get_thinking_level(&self, session_id: &SessionId) -> Result<ThinkingLevel, String>"
    assert!(
        body.contains("fn get_thinking_level(&self, session_id: &SessionId) -> Result<ThinkingLevel, String>"),
        "SessionManagerHandle must declare `fn get_thinking_level(&self, session_id: &SessionId) -> Result<ThinkingLevel, String>`"
    );
    // @step And both methods have default implementations that refuse (BUG-445, BUG-464)
    assert!(
        body.contains("get_model_info is not implemented by this session manager"),
        "SessionManagerHandle default impl for get_model_info must refuse"
    );
    assert!(
        body.contains("get_thinking_level is not implemented by this session manager"),
        "SessionManagerHandle default impl for get_thinking_level must refuse"
    );
    Ok(())
}

/// Scenario: FspecBackend trait declares the three new methods
#[test]
fn fspec_backend_trait_declares_the_three_new_methods() -> TestResult {
    // @step Given rust/fspec-tui/src/transport/mod.rs after RPC-018 lands
    let body = read_raw(&src_dir()?.join("transport").join("mod.rs"))?;
    // @step Then the file contains the substring "async fn get_model_info"
    assert!(body.contains("async fn get_model_info"));
    // @step And the file contains the substring "async fn get_thinking_level"
    assert!(body.contains("async fn get_thinking_level"));
    // @step And the file contains the substring "async fn get_workspace_info"
    assert!(body.contains("async fn get_workspace_info"));
    Ok(())
}

/// Scenario: Both transports implement the three new FspecBackend methods
#[test]
fn both_transports_implement_the_three_new_methods() -> TestResult {
    // @step Given the rust/fspec-tui crate after RPC-018 lands
    let embedded = read_raw(&src_dir()?.join("transport").join("embedded.rs"))?;
    let websocket = read_raw(&src_dir()?.join("transport").join("websocket.rs"))?;
    // @step Then rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_model_info"
    assert!(embedded.contains("async fn get_model_info"));
    // @step And rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_thinking_level"
    assert!(embedded.contains("async fn get_thinking_level"));
    // @step And rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_workspace_info"
    assert!(embedded.contains("async fn get_workspace_info"));
    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_model_info"
    assert!(websocket.contains("async fn get_model_info"));
    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_thinking_level"
    assert!(websocket.contains("async fn get_thinking_level"));
    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_workspace_info"
    assert!(websocket.contains("async fn get_workspace_info"));
    Ok(())
}

/// Scenario: New agent widget modules exist as separate files
#[test]
fn new_agent_widget_modules_exist_as_separate_files() -> TestResult {
    // @step Given the rust/fspec-tui crate after RPC-018 lands
    let header = src_dir()?.join("views").join("agent").join("header.rs");
    let footer = src_dir()?.join("views").join("agent").join("footer.rs");
    // @step Then the file rust/fspec-tui/src/views/agent/header.rs exists
    assert!(
        header.exists(),
        "views/agent/header.rs must exist after RPC-018"
    );
    // @step And the file rust/fspec-tui/src/views/agent/footer.rs exists
    assert!(
        footer.exists(),
        "views/agent/footer.rs must exist after RPC-018"
    );
    Ok(())
}

/// Scenario: New and modified agent modules stay under 300 lines
#[test]
fn new_and_modified_agent_modules_stay_under_300_lines() -> TestResult {
    // @step Given the directory rust/fspec-tui/src/views/agent/ plus the views/agent.rs orchestrator (or views/agent/mod.rs)
    let agent_dir = src_dir()?.join("views").join("agent");
    let agent_orchestrator_rs = src_dir()?.join("views").join("agent.rs");
    let agent_mod_rs = agent_dir.join("mod.rs");

    // @step When a test counts the physical lines of every .rs file
    let mut targets: Vec<std::path::PathBuf> = Vec::new();
    if agent_orchestrator_rs.exists() {
        targets.push(agent_orchestrator_rs.clone());
    }
    if agent_dir.exists() {
        let entries = std::fs::read_dir(&agent_dir)
            .map_err(|e| format!("read_dir {}: {e}", agent_dir.display()))?;
        for entry in entries {
            let entry = entry.or_fail("read_dir entry")?;
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
        let lines = count_lines_path(path)?;
        // @step Then every file in views/agent/ has fewer than 300 lines
        // @step And the orchestrator file (views/agent.rs OR views/agent/mod.rs) has fewer than 300 lines
        if lines >= 300 {
            violations.push(format!(
                "{}: {} lines >= 300 ceiling",
                path.display(),
                lines
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "RPC-018 agent modules MUST stay < 300 LoC. Violations: {violations:?}"
    );
    Ok(())
}

/// Scenario: Action enum gains three new variants
#[test]
fn action_enum_gains_three_new_variants() -> TestResult {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-018 lands
    let body = read_raw(&src_dir()?.join("components").join("mod.rs"))?;
    // @step Then the file contains the substring "ModelInfoLoaded"
    assert!(body.contains("ModelInfoLoaded"));
    // @step And the file contains the substring "ThinkingLevelLoaded"
    assert!(body.contains("ThinkingLevelLoaded"));
    // @step And the file contains the substring "WorkspaceInfoLoaded"
    assert!(body.contains("WorkspaceInfoLoaded"));
    Ok(())
}

/// Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
#[test]
fn views_still_avoid_encapsulated_transport_crates_and_runtime_construction() -> TestResult {
    // @step Given the directory rust/fspec-tui/src/views/ (including views/agent/) after RPC-018 lands
    let views_dir = src_dir()?.join("views");
    // @step When a test scans every *.rs file
    let rs_files = common::collect_rs_files(&views_dir);
    assert!(!rs_files.is_empty(), "expected views/*.rs files");
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_fail(path)?;
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
    Ok(())
}
