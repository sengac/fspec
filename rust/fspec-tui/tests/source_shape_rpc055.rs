//! RPC-055 — Source-shape assertions for the /debug RPC surface.
//!
//! Feature: spec/features/rpc055-slash-debug-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the `set_debug_directory` RPC method and the `/debug`
//! slash-command dispatch routing. Mirrors the source_shape_rpc054
//! pattern.

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

/// Scenario: SessionManagerHandle declares set_debug_directory
#[test]
fn session_manager_handle_declares_set_debug_directory() -> TestResult {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root()?.join("rust/core/src/session_manager_handle.rs");
    let source = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .or_fail("read session_manager_handle.rs")?;

    // @step Then it declares a trait method named "set_debug_directory" that takes a PathBuf and returns Result<(), String>
    assert!(
        source.contains("fn set_debug_directory(") && source.contains("PathBuf"),
        "session_manager_handle.rs should declare fn set_debug_directory(... PathBuf ...)"
    );
    // The trait method's return type is `Result<(), String>` — assert
    // the full declaration is in the file (regardless of formatting).
    let normalised = source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        normalised
            .contains("fn set_debug_directory( &self, path: PathBuf, ) -> Result<(), String>")
            || normalised
                .contains("fn set_debug_directory(&self, path: PathBuf) -> Result<(), String>"),
        "session_manager_handle.rs should declare set_debug_directory(&self, path: PathBuf) -> Result<(), String>; got normalised={normalised:?}"
    );
    Ok(())
}

/// Scenario: FspecService declares set_debug_directory
#[test]
fn fspec_service_declares_set_debug_directory() -> TestResult {
    // @step Given the file rust/rpc/src/lib.rs is compiled
    let path = workspace_root()?.join("rust/rpc/src/lib.rs");
    let source = read_source(&path).or_fail("read rpc/src/lib.rs")?;

    // @step Then it declares an async fn named "set_debug_directory" with parameter type String and return type Result<(), String>
    assert!(
        source.contains("async fn set_debug_directory("),
        "rpc/src/lib.rs should declare async fn set_debug_directory"
    );
    assert!(
        source.contains("set_debug_directory(path: String) -> Result<(), String>")
            || source.contains(
                "set_debug_directory(\n        path: String,\n    ) -> Result<(), String>"
            ),
        "rpc/src/lib.rs should declare set_debug_directory(path: String) -> Result<(), String>"
    );
    Ok(())
}

/// Scenario: FspecBackend declares set_debug_directory
#[test]
fn fspec_backend_declares_set_debug_directory() -> TestResult {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root()?.join("rust/fspec-tui/src/transport/mod.rs");
    let source = read_source(&path).or_fail("read transport/mod.rs")?;

    // @step Then it declares an async fn named "set_debug_directory" on the FspecBackend trait
    assert!(
        source.contains("async fn set_debug_directory("),
        "transport/mod.rs should declare async fn set_debug_directory"
    );
    Ok(())
}

/// Scenario: Both transports implement set_debug_directory
#[test]
fn both_transports_implement_set_debug_directory() -> TestResult {
    // @step Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    let embedded = read_source(&workspace_root()?.join("rust/fspec-tui/src/transport/embedded.rs"))
        .or_fail("read transport/embedded.rs")?;
    let websocket =
        read_source(&workspace_root()?.join("rust/fspec-tui/src/transport/websocket.rs"))
            .or_fail("read transport/websocket.rs")?;

    // @step Then each file contains an impl of "set_debug_directory" that calls the corresponding tarpc client method
    assert!(
        embedded.contains("async fn set_debug_directory("),
        "embedded.rs should impl set_debug_directory"
    );
    assert!(
        embedded.contains(".set_debug_directory("),
        "embedded.rs should forward to the tarpc client's set_debug_directory"
    );
    assert!(
        websocket.contains("async fn set_debug_directory("),
        "websocket.rs should impl set_debug_directory"
    );
    assert!(
        websocket.contains(".set_debug_directory("),
        "websocket.rs should forward to the tarpc client's set_debug_directory"
    );
    Ok(())
}

/// Scenario: /debug slash command wiring lives in dispatch_slash_debug.rs
#[test]
fn slash_debug_wiring_lives_in_dispatch_slash_debug() -> TestResult {
    // @step Given the file rust/fspec-tui/src/app/dispatch_slash_debug.rs exists
    let source =
        read_source(&workspace_root()?.join("rust/fspec-tui/src/app/dispatch_slash_debug.rs"))
            .or_fail("read app/dispatch_slash_debug.rs")?;

    // @step Then it declares a method named "handle_slash_debug"
    assert!(
        source.contains("fn handle_slash_debug("),
        "dispatch_slash_debug.rs should declare fn handle_slash_debug"
    );
    Ok(())
}
