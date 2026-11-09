//! RPC-056 — Source-shape assertions for the /blocklist RPC surface.
//!
//! Feature: spec/features/rpc056-blocklist-view-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the `blocklist_list` RPC method, the `BlocklistRuleInfo`
//! wire type, the new `BlocklistView` + `ViewMode::Blocklist`, and the
//! `/blocklist` slash-command dispatch routing. Mirrors the
//! source_shape_rpc054 / source_shape_rpc055 patterns.

use codelet_test_helpers::{OrFail, TestResult};
use std::path::PathBuf;

use codelet_test_helpers::source_composition::read_source;

fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .and_then(|p| p.parent())
        .or_fail("workspace root above rust/fspec-tui")?
        .to_path_buf())
}

/// Scenario: BlocklistRuleInfo is exported from codelet-rpc-types
#[test]
fn rpc_types_exports_blocklist_rule_info() -> TestResult {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/rpc-types/src/lib.rs");
    let source = read_tree(&path)?;

    // @step Then it declares a public struct named "BlocklistRuleInfo"
    assert!(
        source.contains("pub struct BlocklistRuleInfo"),
        "rpc-types/src/lib.rs should declare pub struct BlocklistRuleInfo"
    );

    // @step And the struct has fields named id, pattern, action, reason, guidance, source
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    for field in [
        "pub id:",
        "pub pattern:",
        "pub action:",
        "pub reason:",
        "pub guidance:",
        "pub source:",
    ] {
        assert!(
            normalised.contains(field),
            "BlocklistRuleInfo should declare field {field:?}; normalised text was searched"
        );
    }
    Ok(())
}

/// Scenario: SessionManagerHandle declares blocklist_list
#[test]
fn session_manager_handle_declares_blocklist_list() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = read_tree(&path)?;

    // @step Then it declares a trait method named "blocklist_list" that returns Result<Vec<BlocklistRuleInfo>, String>
    assert!(
        source.contains("fn blocklist_list("),
        "session_manager_handle.rs should declare fn blocklist_list"
    );
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalised.contains("fn blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>, String>"),
        "session_manager_handle.rs should declare blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>, String>"
    );
    Ok(())
}

/// Scenario: StubSessionManagerHandle exposes a blocklist_list call counter
#[test]
fn stub_exposes_blocklist_list_call_counter() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = read_tree(&path)?;

    // @step Then StubSessionManagerHandle declares a method named "blocklist_list_calls" returning u64
    assert!(
        source.contains("pub fn blocklist_list_calls("),
        "StubSessionManagerHandle should declare pub fn blocklist_list_calls"
    );
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalised.contains("pub fn blocklist_list_calls(&self) -> u64"),
        "StubSessionManagerHandle should declare blocklist_list_calls(&self) -> u64"
    );
    Ok(())
}

/// Scenario: FspecService declares blocklist_list
#[test]
fn fspec_service_declares_blocklist_list() -> TestResult {
    // @step Given the file rust/rpc/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/rpc/src/lib.rs");
    let source = read_source(&path).or_fail("read rpc/src/lib.rs")?;

    // @step Then it declares an async fn named "blocklist_list" with return type Result<Vec<BlocklistRuleInfo>, String>
    assert!(
        source.contains("async fn blocklist_list("),
        "rpc/src/lib.rs should declare async fn blocklist_list"
    );
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalised.contains("blocklist_list() -> Result<Vec<BlocklistRuleInfo>, String>"),
        "rpc/src/lib.rs should declare blocklist_list() -> Result<Vec<BlocklistRuleInfo>, String>"
    );
    Ok(())
}

/// Scenario: FspecBackend declares blocklist_list
#[test]
fn fspec_backend_declares_blocklist_list() -> TestResult {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/transport/mod.rs");
    let source = read_source(&path).or_fail("read transport/mod.rs")?;

    // @step Then it declares an async fn named "blocklist_list" on the FspecBackend trait returning Result<Vec<BlocklistRuleInfo>>
    assert!(
        source.contains("async fn blocklist_list("),
        "transport/mod.rs should declare async fn blocklist_list"
    );
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalised.contains("blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>>"),
        "transport/mod.rs should declare blocklist_list(&self) -> Result<Vec<BlocklistRuleInfo>>"
    );
    Ok(())
}

/// Scenario: Both transports implement blocklist_list
#[test]
fn both_transports_implement_blocklist_list() -> TestResult {
    // @step Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    let embedded = read_source(&workspace_root()?.join("rust/fspec-tui/src/transport/embedded.rs"))
        .or_fail("read transport/embedded.rs")?;
    let websocket =
        read_source(&workspace_root()?.join("rust/fspec-tui/src/transport/websocket.rs"))
            .or_fail("read transport/websocket.rs")?;

    // @step Then each file contains an impl of "blocklist_list" that calls the corresponding tarpc client method
    assert!(
        embedded.contains("async fn blocklist_list("),
        "embedded.rs should impl blocklist_list"
    );
    assert!(
        embedded.contains(".blocklist_list("),
        "embedded.rs should forward to the tarpc client's blocklist_list"
    );
    assert!(
        websocket.contains("async fn blocklist_list("),
        "websocket.rs should impl blocklist_list"
    );
    assert!(
        websocket.contains(".blocklist_list("),
        "websocket.rs should forward to the tarpc client's blocklist_list"
    );
    Ok(())
}

/// Scenario: BlocklistView module exists with the documented entry points
#[test]
fn blocklist_view_module_exists() -> TestResult {
    // @step Given the file rust/fspec-tui/src/views/blocklist/mod.rs exists
    let path = workspace_root()?.join("rust/fspec-tui/src/views/blocklist/mod.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read views/blocklist/mod.rs")?;

    // @step Then it declares a public struct named "BlocklistView"
    assert!(
        source.contains("pub struct BlocklistView"),
        "views/blocklist/mod.rs should declare pub struct BlocklistView"
    );

    // @step And it declares an enum named "BlocklistEvent" or its rename equivalent
    assert!(
        source.contains("enum BlocklistEvent"),
        "views/blocklist/mod.rs should declare enum BlocklistEvent"
    );

    // @step And it declares a free function named "derive_category" returning &'static str
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalised.contains("fn derive_category(pattern: &str) -> &'static str")
            || normalised.contains("fn derive_category(pattern: &str)-> &'static str"),
        "views/blocklist/mod.rs should declare fn derive_category(pattern: &str) -> &'static str"
    );
    Ok(())
}

/// Scenario: Navigator exposes a ViewMode::Blocklist variant
#[test]
fn navigator_declares_view_mode_blocklist() -> TestResult {
    // @step Given the file rust/fspec-tui/src/views/navigator.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/views/navigator.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read navigator.rs")?;

    // @step Then ViewMode declares a variant named "Blocklist"
    // The variant could be either `Blocklist,` (bare) or `Blocklist {` (struct variant).
    assert!(
        source.contains("Blocklist,")
            || source.contains("Blocklist\n")
            || source.contains("Blocklist {"),
        "navigator.rs should declare a ViewMode::Blocklist variant"
    );
    Ok(())
}

/// Scenario: /blocklist slash command wiring lives in dispatch_blocklist.rs
#[test]
fn blocklist_slash_command_wiring_lives_in_dispatch_blocklist() -> TestResult {
    // @step Given the file rust/fspec-tui/src/app/dispatch_blocklist.rs exists
    let path = workspace_root()?.join("rust/fspec-tui/src/app/dispatch_blocklist.rs");
    let source = read_tree(&path)?;
    // @step Then it declares a method named "handle_open_blocklist_view"
    assert!(source.contains("fn handle_open_blocklist_view("));
    // @step And it declares a method named "handle_close_blocklist_view"
    assert!(source.contains("fn handle_close_blocklist_view("));
    // @step And it declares a method named "handle_blocklist_rules_loaded"
    assert!(source.contains("fn handle_blocklist_rules_loaded("));
    // @step And it declares a method named "try_dispatch_blocklist"
    assert!(source.contains("fn try_dispatch_blocklist("));
    Ok(())
}

/// REFAC-021/022: read a source file together with its include!d fragments
/// and file-backed child modules (split modules read as one text).
fn read_tree(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_module_tree(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?,
    )
}
