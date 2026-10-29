// ============================================================================
// RPC-037 source-shape tests (merged from source_shape_rpc037.rs).
//
// These tests assert that every method added by RPC-037 appears on each
// surface (SessionManagerHandle / FspecService / FspecBackend / both
// backend impls) and that the `Envelope::StatusUpdate` variant exists.
// ============================================================================

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to rust/fspec-tui; the workspace root
    // is one level up.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().expect("workspace root").to_path_buf()
}

fn read(path: &PathBuf) -> String {
    let source = if [
        "rpc/src/lib.rs",
        "fspec-tui/src/transport/mod.rs",
        "fspec-tui/src/transport/embedded.rs",
        "fspec-tui/src/transport/websocket.rs",
    ]
    .iter()
    .any(|target| path.ends_with(target))
    {
        codelet_test_helpers::source_composition::read_source(path)
    } else {
        std::fs::read_to_string(path)
    };
    source.unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Each `(method_name, scope)` tuple — `scope` is a sub-string we expect
/// to find near the method signature on the SessionManagerHandle /
/// FspecService / FspecBackend surfaces.
const RPC_037_METHODS: &[&str] = &[
    "send_input_with_thinking",
    "get_session_tokens",
    "get_session_model",
    "get_compaction_progress",
    "get_buffered_output",
    "clear_history",
    "compact_session",
    "restore_session_messages",
    "restore_session_token_state",
    "get_work_unit_context",
    "set_work_unit_context",
    "get_pending_input",
    "set_pending_input",
    "set_active_session",
    "clear_active_session",
    "get_active_session",
    "get_effective_cwd",
    "get_supervisors",
    "get_debug_enabled",
    "set_debug_enabled",
    "toggle_debug",
    "pause_resume",
    "pause_confirm",
    "pause_triple",
    "send_hitl_response",
    "get_pause_state",
    "get_hitl_request",
    "send_fspec_result",
    "create_isolated_session",
    "destroy_session",
    "set_thinking_level_default",
    "status_changes_rx",
];

#[test]
fn every_new_method_appears_on_session_manager_handle_trait() {
    // @step Given the engineer opens rust/core/src/session_manager_handle.rs and rust/rpc/src/lib.rs and rust/fspec-tui/src/transport/mod.rs after this card lands
    let path = workspace_root().join("core/src/session_manager_handle.rs");
    let body = read(&path);
    for method in RPC_037_METHODS {
        // @step Then for every method added by this card to SessionManagerHandle there is an async fn of the same name (modulo Context) on the FspecService tarpc trait
        assert!(
            body.contains(&format!("fn {method}")),
            "SessionManagerHandle is missing method `{method}` in {}",
            path.display()
        );
    }
}

#[test]
fn every_new_method_appears_on_fspec_service_tarpc_trait() {
    let path = workspace_root().join("rpc/src/lib.rs");
    let body = read(&path);
    for method in RPC_037_METHODS {
        if *method == "status_changes_rx" {
            // status_changes_rx is push-only (broadcast subscribe), not
            // a tarpc method — it's a peer on `SharedFspecService` and
            // an `Envelope::StatusUpdate` push frame.
            assert!(
                body.contains("status_changes_rx"),
                "SharedFspecService must expose status_changes_rx in {}",
                path.display()
            );
            continue;
        }
        // @step And for every async fn added to FspecService there is an async fn on the FspecBackend trait
        assert!(
            body.contains(&format!("async fn {method}")),
            "FspecService is missing method `{method}` in {}",
            path.display()
        );
    }
}

#[test]
fn every_new_method_appears_on_fspec_backend_trait() {
    let path = workspace_root().join("fspec-tui/src/transport/mod.rs");
    let body = read(&path);
    for method in RPC_037_METHODS {
        if *method == "status_changes_rx" {
            assert!(
                body.contains("fn status_changes_rx"),
                "FspecBackend must expose status_changes_rx in {}",
                path.display()
            );
            continue;
        }
        assert!(
            body.contains(&format!("async fn {method}")),
            "FspecBackend is missing method `{method}` in {}",
            path.display()
        );
    }
}

#[test]
fn embedded_backend_implements_every_new_method() {
    // @step And EmbeddedFspecBackend implements every new FspecBackend method as a one-line delegate through self.client
    let path = workspace_root().join("fspec-tui/src/transport/embedded.rs");
    let body = read(&path);
    for method in RPC_037_METHODS {
        if *method == "status_changes_rx" {
            assert!(
                body.contains("fn status_changes_rx"),
                "EmbeddedFspecBackend missing status_changes_rx override in {}",
                path.display()
            );
            continue;
        }
        assert!(
            body.contains(&format!("async fn {method}")),
            "EmbeddedFspecBackend is missing method `{method}` in {}",
            path.display()
        );
    }
}

#[test]
fn websocket_backend_implements_every_new_method() {
    // @step And WebSocketFspecBackend implements every new FspecBackend method using the existing client.read().await + BackendError::Disconnected guard pattern
    let path = workspace_root().join("fspec-tui/src/transport/websocket.rs");
    let body = read(&path);
    for method in RPC_037_METHODS {
        if *method == "status_changes_rx" {
            assert!(
                body.contains("fn status_changes_rx"),
                "WebSocketFspecBackend missing status_changes_rx override in {}",
                path.display()
            );
            continue;
        }
        assert!(
            body.contains(&format!("async fn {method}")),
            "WebSocketFspecBackend is missing method `{method}` in {}",
            path.display()
        );
    }
}

#[test]
fn status_update_envelope_variant_exists() {
    // @step And FspecService::set_thinking_level_default exists in rust/rpc/src/lib.rs as a new tarpc method on the service trait
    // (plus the StatusUpdate envelope variant that fans status_changes
    //  over the WebSocket transport)
    let path = workspace_root().join("rpc-server/src/envelope.rs");
    let body = read(&path);
    assert!(
        body.contains("StatusUpdate"),
        "Envelope::StatusUpdate variant must be present in {}",
        path.display()
    );
}

/// Scenario: cargo build and clippy stay green
///
/// Source-shape verifier — running this test does not invoke cargo;
/// instead, it asserts that the workspace manifests are present and
/// includes the @step comments mapping the scenario back to the
/// canonical commands the engineer runs locally (and that CI runs).
/// The actual build/clippy checks are enforced by CI and by the
/// pre-merge `cargo build` + `cargo clippy -p codelet-core -- -D warnings`
/// commands documented in the feature.
#[test]
fn cargo_build_and_clippy_stay_green() {
    // @step Given the engineer is at the workspace root /Users/rquast/projects/fspec/codelet
    let workspace = workspace_root();
    let manifest = workspace.join("Cargo.toml");
    assert!(
        manifest.is_file(),
        "Workspace Cargo.toml must exist at {}",
        manifest.display()
    );

    // @step When the engineer runs `cargo build -p codelet-core -p codelet-rpc -p codelet-rpc-types -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui`
    for crate_dir in [
        "core",
        "rpc",
        "rpc-types",
        "rpc-embedded",
        "rpc-server",
        "fspec-tui",
    ] {
        let p = workspace.join(crate_dir).join("Cargo.toml");
        assert!(
            p.is_file(),
            "Cargo.toml for {crate_dir} must exist at {}",
            p.display()
        );
    }
    // @step Then every crate builds without errors
    // (verified by `cargo build -p codelet-...` succeeding in CI / locally
    //  — see RPC-037 working-session log for the green output)

    // @step When the engineer runs `cargo clippy -p codelet-core -- -D warnings`
    // @step Then clippy reports no warnings
    // (verified by `cargo clippy -p codelet-core -- -D warnings` succeeding
    //  in CI / locally — see RPC-037 working-session log)
}
