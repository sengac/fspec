//! RPC-022 — Source-shape regression for the modal dialogs port +
//! shared types + new RPC methods.
//!
//! Feature: spec/features/rpc022-source-shape.feature
//!
//! Mirrors `source_shape_rpc018.rs`: pure path + substring tests that
//! pin the source layout introduced by RPC-022 so future refactors
//! cannot silently relocate ProviderInfo / ModelEntry / the five new
//! RPC methods / the new Priority::Foreground variant.

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
    common::workspace_root()
        .join("core")
        .join("src")
        .join("session_manager_handle.rs")
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
    // @step Given codelet/rpc-types/src/lib.rs after RPC-022 lands
    let src = read_raw(&rpc_types_lib());
    // @step Then the file contains the substring "pub struct ProviderInfo"
    assert!(src.contains("pub struct ProviderInfo"));
    // @step And the file contains the substring "pub key: String"
    assert!(src.contains("pub key: String"));
    // @step And the file contains the substring "pub display_name: String"
    assert!(src.contains("pub display_name: String"));
    // @step And the file contains the substring "pub models: Vec<ModelEntry>"
    assert!(src.contains("pub models: Vec<ModelEntry>"));
    // @step And the file contains the substring "pub struct ModelEntry"
    assert!(src.contains("pub struct ModelEntry"));
    // @step And the file contains the substring "pub id: String"
    assert!(src.contains("pub id: String"));
    // @step And the file contains the substring "pub context_window: u32"
    assert!(src.contains("pub context_window: u32"));
    // @step And the file contains the substring "pub supports_reasoning: bool"
    assert!(src.contains("pub supports_reasoning: bool"));
    // @step And the file contains the substring "pub supports_vision: bool"
    assert!(src.contains("pub supports_vision: bool"));
    // @step And the file contains the substring "pub is_custom: bool"
    assert!(src.contains("pub is_custom: bool"));
}

/// Scenario: FspecService trait gains five new RPC methods
#[test]
fn fspec_service_trait_gains_five_new_rpc_methods() {
    // @step Given codelet/rpc/src/lib.rs after RPC-022 lands
    let raw = read_raw(&rpc_lib());
    // Whitespace-collapsed view so the multi-line trait declarations
    // (which the rustfmt'd file wraps across 4-5 lines per method)
    // still match the canonical Gherkin signature.
    let collapsed: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    // @step Then the file contains the substring "async fn list_providers() -> Vec<ProviderInfo>"
    assert!(collapsed.contains("async fn list_providers() -> Vec<ProviderInfo>"));
    // @step And the file contains the substring "async fn set_session_model(session_id: SessionId, provider_id: String, model_id: String) -> Result<(), String>"
    assert!(collapsed.contains(
        "async fn set_session_model( session_id: SessionId, provider_id: String, model_id: String, ) -> Result<(), String>"
    ) || collapsed.contains(
        "async fn set_session_model(session_id: SessionId, provider_id: String, model_id: String) -> Result<(), String>"
    ));
    // @step And the file contains the substring "async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>"
    assert!(collapsed.contains(
        "async fn set_thinking_level( session_id: SessionId, level: ThinkingLevel, ) -> Result<(), String>"
    ) || collapsed.contains(
        "async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>"
    ));
    // @step And the file contains the substring "async fn get_session_role(session_id: SessionId) -> Option<String>"
    assert!(collapsed.contains("async fn get_session_role(session_id: SessionId) -> Option<String>"));
    // @step And the file contains the substring "async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>"
    assert!(collapsed.contains(
        "async fn set_session_role( session_id: SessionId, role: Option<String>, ) -> Result<(), String>"
    ) || collapsed.contains(
        "async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>"
    ));
}

/// Scenario: SessionManagerHandle trait gains the new methods with default impls
#[test]
fn session_manager_handle_trait_gains_new_methods_with_default_impls() {
    // @step Given codelet/core/src/session_manager_handle.rs after RPC-022 lands
    let src = read_raw(&core_session_manager_handle());
    // @step Then the file contains the substring "fn list_providers(&self) -> Vec<ProviderInfo>"
    assert!(src.contains("fn list_providers(&self) -> Vec<ProviderInfo>"));
    // @step And the file contains the substring "fn set_model(&self, session_id: &SessionId, provider_id: &str, model_id: &str) -> Result<(), String>"
    assert!(src.contains(
        "fn set_model("
    ));
    assert!(src.contains("session_id: &SessionId"));
    assert!(src.contains("provider_id: &str"));
    assert!(src.contains("model_id: &str"));
    // @step And the file contains the substring "fn set_thinking_level(&self, session_id: &SessionId, level: ThinkingLevel) -> Result<(), String>"
    assert!(src.contains("fn set_thinking_level("));
    assert!(src.contains("level: ThinkingLevel"));
    // @step And the file contains the substring "fn get_role(&self, session_id: &SessionId) -> Option<String>"
    assert!(src.contains("fn get_role(&self, session_id: &SessionId) -> Option<String>"));
    // @step And the file contains the substring "fn set_role(&self, session_id: &SessionId, role: Option<String>) -> Result<(), String>"
    assert!(src.contains("fn set_role("));
    assert!(src.contains("role: Option<String>"));
    // @step And each of the five methods has a default implementation returning the safe default (empty Vec / None / Ok(()))
    assert!(src.contains("Vec::new()"), "list_providers default = Vec::new()");
    // Each defaulted setter body should end with Ok(()), and get_role returns None.
    let occurrences_ok_unit = src.matches("Ok(())").count();
    assert!(
        occurrences_ok_unit >= 3,
        "expected >=3 Ok(()) defaults (set_model/set_thinking_level/set_role), got {occurrences_ok_unit}"
    );
}

/// Scenario: FspecBackend trait declares the five new methods
#[test]
fn fspec_backend_trait_declares_five_new_methods() {
    // @step Given codelet/fspec-tui/src/transport/mod.rs after RPC-022 lands
    let src = read_raw(&src_dir().join("transport").join("mod.rs"));
    // @step Then the file contains the substring "async fn list_providers"
    assert!(src.contains("async fn list_providers"));
    // @step And the file contains the substring "async fn set_session_model"
    assert!(src.contains("async fn set_session_model"));
    // @step And the file contains the substring "async fn set_thinking_level"
    assert!(src.contains("async fn set_thinking_level"));
    // @step And the file contains the substring "async fn get_session_role"
    assert!(src.contains("async fn get_session_role"));
    // @step And the file contains the substring "async fn set_session_role"
    assert!(src.contains("async fn set_session_role"));
}

/// Scenario: Both transports implement the five new FspecBackend methods
#[test]
fn both_transports_implement_five_new_backend_methods() {
    // @step Given the codelet/fspec-tui crate after RPC-022 lands
    let embedded = read_raw(&src_dir().join("transport").join("embedded.rs"));
    let websocket = read_raw(&src_dir().join("transport").join("websocket.rs"));
    // @step Then codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn list_providers"
    assert!(embedded.contains("async fn list_providers"));
    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn set_session_model"
    assert!(embedded.contains("async fn set_session_model"));
    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn set_thinking_level"
    assert!(embedded.contains("async fn set_thinking_level"));
    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn get_session_role"
    assert!(embedded.contains("async fn get_session_role"));
    // @step And codelet/fspec-tui/src/transport/embedded.rs contains the substring "async fn set_session_role"
    assert!(embedded.contains("async fn set_session_role"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn list_providers"
    assert!(websocket.contains("async fn list_providers"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn set_session_model"
    assert!(websocket.contains("async fn set_session_model"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn set_thinking_level"
    assert!(websocket.contains("async fn set_thinking_level"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn get_session_role"
    assert!(websocket.contains("async fn get_session_role"));
    // @step And codelet/fspec-tui/src/transport/websocket.rs contains the substring "async fn set_session_role"
    assert!(websocket.contains("async fn set_session_role"));
}

/// Scenario: New modal dialog modules and dispatch helper exist
#[test]
fn new_modal_dialog_modules_and_dispatch_helper_exist() {
    // @step Given the codelet/fspec-tui crate after RPC-022 lands
    let src = src_dir();
    // @step Then the file codelet/fspec-tui/src/components/model_selector_dialog.rs exists
    assert!(src.join("components").join("model_selector_dialog.rs").exists());
    // @step And the file codelet/fspec-tui/src/components/thinking_level_dialog.rs exists
    assert!(src.join("components").join("thinking_level_dialog.rs").exists());
    // @step And the file codelet/fspec-tui/src/views/agent/role_banner.rs exists
    assert!(src.join("views").join("agent").join("role_banner.rs").exists());
    // @step And the file codelet/fspec-tui/src/app/dispatch_rpc022.rs exists
    assert!(src.join("app").join("dispatch_rpc022.rs").exists());
}

/// Scenario: New RPC-022 modules stay under 300 lines
#[test]
fn new_rpc022_modules_stay_under_300_lines() {
    // @step Given the new files introduced by RPC-022
    let model_selector = src_dir().join("components").join("model_selector_dialog.rs");
    let thinking_level = src_dir().join("components").join("thinking_level_dialog.rs");
    let role_banner = src_dir().join("views").join("agent").join("role_banner.rs");
    let dispatch = src_dir().join("app").join("dispatch_rpc022.rs");
    // @step When a test counts the line-count of every .rs file
    let lines_model = count_lines_path(&model_selector);
    let lines_thinking = count_lines_path(&thinking_level);
    let lines_role = count_lines_path(&role_banner);
    let lines_dispatch = count_lines_path(&dispatch);
    // @step Then codelet/fspec-tui/src/components/model_selector_dialog.rs has fewer than 300 lines
    assert!(
        lines_model < 300,
        "model_selector_dialog.rs has {lines_model} lines (>= 300)"
    );
    // @step And codelet/fspec-tui/src/components/thinking_level_dialog.rs has fewer than 300 lines
    assert!(
        lines_thinking < 300,
        "thinking_level_dialog.rs has {lines_thinking} lines (>= 300)"
    );
    // @step And codelet/fspec-tui/src/views/agent/role_banner.rs has fewer than 300 lines
    assert!(
        lines_role < 300,
        "role_banner.rs has {lines_role} lines (>= 300)"
    );
    // @step And codelet/fspec-tui/src/app/dispatch_rpc022.rs has fewer than 300 lines
    assert!(
        lines_dispatch < 300,
        "dispatch_rpc022.rs has {lines_dispatch} lines (>= 300)"
    );
}

/// Scenario: Priority enum gains a Foreground variant numbered 900
#[test]
fn priority_enum_gains_foreground_variant_numbered_900() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-022 lands
    let src = read_raw(&src_dir().join("components").join("mod.rs"));
    // @step Then the Priority enum contains the variant "Foreground = 900"
    assert!(src.contains("Foreground = 900"));
    // @step And Priority::Foreground sorts strictly between Priority::High (800) and Priority::Critical (1000)
    use codelet_fspec_tui::Priority;
    assert!(Priority::High < Priority::Foreground);
    assert!(Priority::Foreground < Priority::Critical);
}

/// Scenario: Action enum gains the new RPC-022 variants
#[test]
fn action_enum_gains_new_rpc022_variants() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-022 lands
    let src = read_raw(&src_dir().join("components").join("mod.rs"));
    // @step Then the file contains the substring "ModelSelected"
    assert!(src.contains("ModelSelected"));
    // @step And the file contains the substring "ThinkingLevelSelected"
    assert!(src.contains("ThinkingLevelSelected"));
    // @step And the file contains the substring "SetSessionRole"
    assert!(src.contains("SetSessionRole"));
    // @step And the file contains the substring "SessionRoleLoaded"
    assert!(src.contains("SessionRoleLoaded"));
    // @step And the file contains the substring "ListProvidersLoaded"
    assert!(src.contains("ListProvidersLoaded"));
    // @step And the file contains the substring "OpenModelDialog"
    assert!(src.contains("OpenModelDialog"));
    // @step And the file contains the substring "OpenThinkingDialog"
    assert!(src.contains("OpenThinkingDialog"));
}

/// Scenario: Existing TS modal dialog files are untouched
#[test]
fn existing_ts_modal_dialog_files_are_untouched() {
    // @step Given the project root after RPC-022 lands
    let root = project_root();
    // @step Then the file src/tui/components/ModelSelectorScreen.tsx exists
    assert!(root.join("src/tui/components/ModelSelectorScreen.tsx").exists());
    // @step And the file src/tui/components/ModelSelectorView.tsx exists
    assert!(root.join("src/tui/components/ModelSelectorView.tsx").exists());
    // @step And the file src/tui/components/ThinkingLevelDialog.tsx exists
    assert!(root.join("src/tui/components/ThinkingLevelDialog.tsx").exists());
    // @step And the file src/tui/components/RoleBanner.tsx exists
    assert!(root.join("src/tui/components/RoleBanner.tsx").exists());
    // @step And the file src/tui/store/modelStore.ts exists
    assert!(root.join("src/tui/store/modelStore.ts").exists());
}

/// Scenario: New view + component files do not directly import codelet_core / napi / tarpc / tokio_tungstenite
#[test]
fn new_view_component_files_do_not_import_forbidden_crates() {
    // @step Given the new RPC-022 files (model_selector_dialog.rs, thinking_level_dialog.rs, role_banner.rs)
    let files = [
        src_dir().join("components").join("model_selector_dialog.rs"),
        src_dir().join("components").join("thinking_level_dialog.rs"),
        src_dir().join("views").join("agent").join("role_banner.rs"),
    ];
    // @step When a test scans each *.rs file
    for path in files {
        let src = common::strip_rust_comments(&read_raw(&path));
        // @step Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
        assert!(
            !src.contains("codelet_core::"),
            "{} imports codelet_core::",
            path.display()
        );
        assert!(
            !src.contains("codelet_napi::"),
            "{} imports codelet_napi::",
            path.display()
        );
        assert!(
            !src.contains("tarpc::"),
            "{} imports tarpc::",
            path.display()
        );
        assert!(
            !src.contains("tokio_tungstenite::"),
            "{} imports tokio_tungstenite::",
            path.display()
        );
        // @step And no file constructs `tokio::runtime::Builder` or `Runtime::new()`
        assert!(
            !src.contains("tokio::runtime::Builder"),
            "{} constructs tokio::runtime::Builder",
            path.display()
        );
        assert!(
            !src.contains("Runtime::new("),
            "{} constructs Runtime::new(...)",
            path.display()
        );
    }
}
