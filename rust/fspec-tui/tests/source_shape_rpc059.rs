//! RPC-059 — Source-shape assertions for the /loop RPC surface.
//!
//! Feature: spec/features/rpc059-loop-source-shape.feature
//!
//! These tests scan source files at compile time to pin the layering
//! contract for the THREE new RPC methods (loop_add, loop_cancel,
//! loop_list), the NEW wire type (RegisteredLoop), the new
//! LoopSubcommand parser, and the `/loop` slash-command dispatch
//! routing in `dispatch_slash_loop.rs`. Mirrors the source_shape_rpc058
//! pattern.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above rust/fspec-tui")
        .to_path_buf()
}

fn normalise(source: &str) -> String {
    source
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Scenario: RegisteredLoop wire type is exported from codelet-rpc-types
#[test]
fn rpc_types_exports_registered_loop_wire_type() {
    // @step Given the file rust/rpc-types/src/lib.rs is compiled
    let path = workspace_root().join("rust/rpc-types/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc-types/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares a public struct named "RegisteredLoop"
    assert!(
        source.contains("pub struct RegisteredLoop"),
        "rpc-types/src/lib.rs should declare pub struct RegisteredLoop"
    );

    // @step And RegisteredLoop has fields named id, session_id, prompt, interval_seconds
    // @step And RegisteredLoop has fields named created_at, expires_at, last_run_at
    for field in [
        "pub id:",
        "pub session_id:",
        "pub prompt:",
        "pub interval_seconds:",
        "pub created_at:",
        "pub expires_at:",
        "pub last_run_at:",
    ] {
        assert!(
            normalised.contains(field),
            "RegisteredLoop should declare field {field:?}"
        );
    }
}

/// Scenario: SessionManagerHandle declares the three new loop methods
#[test]
fn session_manager_handle_declares_loop_methods() {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("rust/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");
    let normalised = normalise(&source);

    // @step Then it declares a trait method named "loop_add" returning Result<RegisteredLoop, String>
    assert!(
        source.contains("fn loop_add("),
        "session_manager_handle.rs should declare fn loop_add"
    );
    assert!(
        normalised.contains("-> Result<RegisteredLoop, String>"),
        "loop_add should return Result<RegisteredLoop, String>"
    );

    // @step And it declares a trait method named "loop_cancel" returning Result<bool, String>
    assert!(
        source.contains("fn loop_cancel("),
        "session_manager_handle.rs should declare fn loop_cancel"
    );
    assert!(
        normalised.contains("-> Result<bool, String>"),
        "loop_cancel should return Result<bool, String>"
    );

    // @step And it declares a trait method named "loop_list" returning Vec<RegisteredLoop>
    assert!(
        source.contains("fn loop_list("),
        "session_manager_handle.rs should declare fn loop_list"
    );
    assert!(
        normalised.contains("-> Vec<RegisteredLoop>"),
        "loop_list should return Vec<RegisteredLoop>"
    );
}

/// Scenario: StubSessionManagerHandle exposes per-call counters for all three loop methods
#[test]
fn stub_exposes_per_call_counters_for_loop() {
    // @step Given the file rust/core/src/session_manager_handle.rs is compiled
    let path = workspace_root().join("rust/core/src/session_manager_handle.rs");
    let source = fs::read_to_string(&path).expect("read session_manager_handle.rs");
    let normalised = normalise(&source);

    // @step Then StubSessionManagerHandle declares a method named "loop_add_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "loop_cancel_calls" returning u64
    // @step And StubSessionManagerHandle declares a method named "loop_list_calls" returning u64
    for counter in ["loop_add_calls", "loop_cancel_calls", "loop_list_calls"] {
        let needle = format!("pub fn {counter}(");
        assert!(
            source.contains(&needle),
            "StubSessionManagerHandle should declare pub fn {counter}"
        );
        let sig = format!("pub fn {counter}(&self) -> u64");
        assert!(
            normalised.contains(&sig),
            "StubSessionManagerHandle should declare {counter}(&self) -> u64"
        );
    }
}

/// Scenario: FspecService declares the three new RPC methods
#[test]
fn fspec_service_declares_loop_methods() {
    // @step Given the file rust/rpc/src/lib.rs is compiled
    let path = workspace_root().join("rust/rpc/src/lib.rs");
    let source = fs::read_to_string(&path).expect("read rpc/src/lib.rs");
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "loop_add" with return type Result<RegisteredLoop, String>
    // @step And it declares an async fn named "loop_cancel" with return type Result<bool, String>
    // @step And it declares an async fn named "loop_list" with return type Vec<RegisteredLoop>
    for method in ["loop_add", "loop_cancel", "loop_list"] {
        let needle = format!("async fn {method}(");
        assert!(
            source.contains(&needle),
            "rpc/src/lib.rs should declare async fn {method}"
        );
    }

    assert!(
        normalised.contains("-> Result<RegisteredLoop, String>"),
        "loop_add should return Result<RegisteredLoop, String>"
    );
    assert!(
        normalised.contains("-> Result<bool, String>"),
        "loop_cancel should return Result<bool, String>"
    );
    assert!(
        normalised.contains("-> Vec<RegisteredLoop>"),
        "loop_list should return Vec<RegisteredLoop>"
    );
}

/// Scenario: FspecBackend declares the three new methods
#[test]
fn fspec_backend_declares_loop_methods() {
    // @step Given the file rust/fspec-tui/src/transport/mod.rs is compiled
    let path = workspace_root().join("rust/fspec-tui/src/transport/mod.rs");
    let source = fs::read_to_string(&path).expect("read transport/mod.rs");
    let normalised = normalise(&source);

    // @step Then it declares an async fn named "loop_add" on the FspecBackend trait returning Result<RegisteredLoop>
    // @step And it declares an async fn named "loop_cancel" on the FspecBackend trait returning Result<bool>
    // @step And it declares an async fn named "loop_list" on the FspecBackend trait returning Result<Vec<RegisteredLoop>>
    for method in ["loop_add", "loop_cancel", "loop_list"] {
        let needle = format!("async fn {method}(");
        assert!(
            source.contains(&needle),
            "transport/mod.rs should declare async fn {method} on FspecBackend"
        );
    }
    assert!(
        normalised.contains("-> Result<RegisteredLoop>"),
        "FspecBackend::loop_add should return Result<RegisteredLoop>"
    );
    assert!(
        normalised.contains("-> Result<bool>"),
        "FspecBackend::loop_cancel should return Result<bool>"
    );
    assert!(
        normalised.contains("-> Result<Vec<RegisteredLoop>>"),
        "FspecBackend::loop_list should return Result<Vec<RegisteredLoop>>"
    );
}

/// Scenario: Both transports implement the three new methods
#[test]
fn both_transports_implement_loop_methods() {
    // @step Given the files rust/fspec-tui/src/transport/embedded.rs and rust/fspec-tui/src/transport/websocket.rs are compiled
    let embedded =
        fs::read_to_string(workspace_root().join("rust/fspec-tui/src/transport/embedded.rs"))
            .expect("read transport/embedded.rs");
    let websocket =
        fs::read_to_string(workspace_root().join("rust/fspec-tui/src/transport/websocket.rs"))
            .expect("read transport/websocket.rs");

    // @step Then each file contains an impl of "loop_add" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "loop_cancel" that calls the corresponding tarpc client method
    // @step And each file contains an impl of "loop_list" that calls the corresponding tarpc client method
    for method in ["loop_add", "loop_cancel", "loop_list"] {
        let impl_needle = format!("async fn {method}(");
        let forward_needle = format!(".{method}(");
        assert!(
            embedded.contains(&impl_needle),
            "embedded.rs should impl {method}"
        );
        assert!(
            embedded.contains(&forward_needle),
            "embedded.rs should forward to the tarpc client's {method}"
        );
        assert!(
            websocket.contains(&impl_needle),
            "websocket.rs should impl {method}"
        );
        assert!(
            websocket.contains(&forward_needle),
            "websocket.rs should forward to the tarpc client's {method}"
        );
    }
}

/// Scenario: loop_parser module exists with the documented entry points
#[test]
fn loop_parser_module_exists() {
    // @step Given the file rust/fspec-tui/src/app/loop_parser.rs exists
    let path = workspace_root().join("rust/fspec-tui/src/app/loop_parser.rs");
    let source = fs::read_to_string(&path).expect("read app/loop_parser.rs");

    // @step Then it declares a public enum named "LoopSubcommand"
    assert!(
        source.contains("pub enum LoopSubcommand"),
        "loop_parser.rs should declare pub enum LoopSubcommand"
    );

    // @step And LoopSubcommand has variants named Add, Cancel, List, Help
    for variant in ["Add", "Cancel", "List", "Help"] {
        assert!(
            source.contains(&format!("{variant},"))
                || source.contains(&format!("{variant} {{"))
                || source.contains(&format!("{variant}(")),
            "LoopSubcommand should declare variant {variant}"
        );
    }

    // @step And it declares a public fn named "parse_loop_command" taking &str and returning LoopSubcommand
    assert!(
        source.contains("pub fn parse_loop_command("),
        "loop_parser.rs should declare pub fn parse_loop_command"
    );
}
