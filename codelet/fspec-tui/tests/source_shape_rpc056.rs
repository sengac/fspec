//! RPC-056 — Source-shape assertions for the /blocklist RPC surface.
//!
//! Feature: spec/features/rpc056-blocklist-view-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the `blocklist_list` RPC method, the `BlocklistRuleInfo`
//! wire type, the new `BlocklistView` + `ViewMode::Blocklist`, and the
//! `/blocklist` slash-command dispatch routing. Mirrors the
//! source_shape_rpc054 / source_shape_rpc055 patterns.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above codelet/fspec-tui")
        .to_path_buf()
}

/// Scenario: BlocklistRuleInfo is exported from codelet-rpc-types
#[test]
fn rpc_types_exports_blocklist_rule_info() {
    // @step Given the file codelet/rpc-types/src/lib.rs is compiled
    let path = workspace_root().join("codelet/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");

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
}

/// Scenario: SessionManagerHandle declares blocklist_list
#[test]
fn session_manager_handle_declares_blocklist_list() {
    // @step Given the file codelet/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("codelet/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");

    // @step Then it declares a trait method named "blocklist_list" that returns Vec<BlocklistRuleInfo>
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
        normalised.contains("fn blocklist_list(&self) -> Vec<BlocklistRuleInfo>")
            || normalised.contains("fn blocklist_list( &self ) -> Vec<BlocklistRuleInfo>"),
        "session_manager_handle.rs should declare blocklist_list(&self) -> Vec<BlocklistRuleInfo>"
    );
}

/// Scenario: StubSessionManagerHandle exposes a blocklist_list call counter
#[test]
fn stub_exposes_blocklist_list_call_counter() {
    // @step Given the file codelet/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("codelet/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");

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
}

/// Scenario: FspecService declares blocklist_list
#[test]
fn fspec_service_declares_blocklist_list() {
    // @step Given the file codelet/rpc/src/lib.rs is compiled
    let path = workspace_root().join("codelet/rpc/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc/src/lib.rs");

    // @step Then it declares an async fn named "blocklist_list" with return type Vec<BlocklistRuleInfo>
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
        normalised.contains("blocklist_list() -> Vec<BlocklistRuleInfo>"),
        "rpc/src/lib.rs should declare blocklist_list() -> Vec<BlocklistRuleInfo>"
    );
}

/// Scenario: FspecBackend declares blocklist_list
#[test]
fn fspec_backend_declares_blocklist_list() {
    // @step Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("codelet/fspec-tui/src/transport/mod.rs");
    let source = fs::read_to_string(&path).expect("read transport/mod.rs");

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
}

/// Scenario: Both transports implement blocklist_list
#[test]
fn both_transports_implement_blocklist_list() {
    // @step Given the files codelet/fspec-tui/src/transport/embedded.rs and codelet/fspec-tui/src/transport/websocket.rs are compiled
    let embedded =
        fs::read_to_string(workspace_root().join("codelet/fspec-tui/src/transport/embedded.rs"))
            .expect("read transport/embedded.rs");
    let websocket =
        fs::read_to_string(workspace_root().join("codelet/fspec-tui/src/transport/websocket.rs"))
            .expect("read transport/websocket.rs");

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
}

/// Scenario: BlocklistView module exists with the documented entry points
#[test]
fn blocklist_view_module_exists() {
    // @step Given the file codelet/fspec-tui/src/views/blocklist/mod.rs exists
    let path = workspace_root().join("codelet/fspec-tui/src/views/blocklist/mod.rs");
    let source = fs::read_to_string(&path).expect("read views/blocklist/mod.rs");

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
}

/// Scenario: Navigator exposes a ViewMode::Blocklist variant
#[test]
fn navigator_declares_view_mode_blocklist() {
    // @step Given the file codelet/fspec-tui/src/views/navigator.rs is compiled
    let path = workspace_root().join("codelet/fspec-tui/src/views/navigator.rs");
    let source = fs::read_to_string(&path).expect("read navigator.rs");

    // @step Then ViewMode declares a variant named "Blocklist"
    // The variant could be either `Blocklist,` (bare) or `Blocklist {` (struct variant).
    assert!(
        source.contains("Blocklist,")
            || source.contains("Blocklist\n")
            || source.contains("Blocklist {"),
        "navigator.rs should declare a ViewMode::Blocklist variant"
    );
}
