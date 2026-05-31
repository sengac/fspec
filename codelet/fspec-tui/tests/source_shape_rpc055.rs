//! RPC-055 — Source-shape assertions for the /debug RPC surface.
//!
//! Feature: spec/features/rpc055-slash-debug-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the `set_debug_directory` RPC method and the `/debug`
//! slash-command dispatch routing. Mirrors the source_shape_rpc054
//! pattern.

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

/// Scenario: SessionManagerHandle declares set_debug_directory
#[test]
fn session_manager_handle_declares_set_debug_directory() {
    // @step Given the file codelet/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("codelet/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");

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
}

/// Scenario: FspecService declares set_debug_directory
#[test]
fn fspec_service_declares_set_debug_directory() {
    // @step Given the file codelet/rpc/src/lib.rs is compiled
    let path = workspace_root().join("codelet/rpc/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc/src/lib.rs");

    // @step Then it declares an async fn named "set_debug_directory" with parameter type String and return type Result<(), String>
    assert!(
        source.contains("async fn set_debug_directory("),
        "rpc/src/lib.rs should declare async fn set_debug_directory"
    );
    assert!(
        source.contains("set_debug_directory(path: String) -> Result<(), String>")
            || source.contains("set_debug_directory(\n        path: String,\n    ) -> Result<(), String>"),
        "rpc/src/lib.rs should declare set_debug_directory(path: String) -> Result<(), String>"
    );
}

/// Scenario: FspecBackend declares set_debug_directory
#[test]
fn fspec_backend_declares_set_debug_directory() {
    // @step Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("codelet/fspec-tui/src/transport/mod.rs");
    let source = fs::read_to_string(&path).expect("read transport/mod.rs");

    // @step Then it declares an async fn named "set_debug_directory" on the FspecBackend trait
    assert!(
        source.contains("async fn set_debug_directory("),
        "transport/mod.rs should declare async fn set_debug_directory"
    );
}

/// Scenario: Both transports implement set_debug_directory
#[test]
fn both_transports_implement_set_debug_directory() {
    // @step Given the files codelet/fspec-tui/src/transport/embedded.rs and codelet/fspec-tui/src/transport/websocket.rs are compiled
    let embedded = fs::read_to_string(
        workspace_root().join("codelet/fspec-tui/src/transport/embedded.rs"),
    )
    .expect("read transport/embedded.rs");
    let websocket = fs::read_to_string(
        workspace_root().join("codelet/fspec-tui/src/transport/websocket.rs"),
    )
    .expect("read transport/websocket.rs");

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
}

/// Scenario: /debug slash command wiring lives in dispatch_rpc055.rs
#[test]
fn dispatch_rpc055_file_has_expected_shape() {
    // @step Given the file codelet/fspec-tui/src/app/dispatch_rpc055.rs exists
    let path = workspace_root().join("codelet/fspec-tui/src/app/dispatch_rpc055.rs");
    let source = fs::read_to_string(&path).expect("read app/dispatch_rpc055.rs");

    // @step Then it declares a method named "handle_slash_debug"
    assert!(
        source.contains("fn handle_slash_debug("),
        "dispatch_rpc055.rs should declare fn handle_slash_debug"
    );

    // @step And it declares a method named "try_dispatch_rpc055"
    assert!(
        source.contains("fn try_dispatch_rpc055("),
        "dispatch_rpc055.rs should declare fn try_dispatch_rpc055"
    );
}
