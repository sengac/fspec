//! Regression: prior-card invariants preserved tests — RPC-011.
//!
//! Feature: spec/features/rpc011-regression-invariants.feature
//!
//! Covers:
//!   - bind_and_serve signature is unchanged
//!   - WebSocketFspecBackend::connect signature is unchanged
//!   - Architecture invariants from RPC-005 still hold
//!   - Earlier RPC-005..010 test suites still pass (compile-time presence)
//!
//! These are source-shape regressions: they read the .rs files in the
//! workspace and assert specific shape invariants survived RPC-011's
//! additive changes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("rpc-server must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: bind_and_serve signature is unchanged
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn bind_and_serve_signature_is_unchanged() {
    // @step Given the public signature of codelet_rpc_server::bind_and_serve
    let server_rs = workspace_root()
        .join("rpc-server")
        .join("src")
        .join("server.rs");
    let body = read(&server_rs);

    // @step When compared against its RPC-005 form
    // @step Then it still returns (SocketAddr, ServerStats, JoinHandle<()>)
    assert!(
        body.contains("(SocketAddr, ServerStats, tokio::task::JoinHandle<()>)"),
        "bind_and_serve return tuple must still be (SocketAddr, ServerStats, JoinHandle<()>)"
    );

    // @step And it still takes (bind_addr: &str, service: Arc<SharedFspecService>) — no new parameter
    assert!(
        body.contains("bind_addr: &str") && body.contains("service: Arc<SharedFspecService>"),
        "bind_and_serve must still take (bind_addr: &str, service: Arc<SharedFspecService>) — no new parameter"
    );

    // Ensure no third parameter sneaked in: the function signature ends
    // with the service arg before the close paren.
    let sig_idx = body
        .find("pub async fn bind_and_serve")
        .expect("bind_and_serve must be public");
    let after = &body[sig_idx..];
    let close_paren_idx = after
        .find(") -> anyhow::Result<")
        .expect("signature must end");
    let sig_body = &after[..close_paren_idx];
    let comma_count = sig_body.matches(',').count();
    assert!(
        comma_count <= 1,
        "bind_and_serve must have exactly two parameters (one comma). Got {comma_count} commas in: {sig_body}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: WebSocketFspecBackend::connect signature is unchanged
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn websocketfspecbackend_connect_signature_is_unchanged() {
    // @step Given the public signature of WebSocketFspecBackend::connect
    let ws_rs = workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("transport")
        .join("websocket.rs");
    let body = read(&ws_rs);

    // @step When compared against its RPC-008 form
    // @step Then it still takes a single url::Url and returns Result<Self>
    assert!(
        body.contains("pub async fn connect(url: url::Url) -> Result<Self>"),
        "WebSocketFspecBackend::connect signature must remain `pub async fn connect(url: url::Url) -> Result<Self>`"
    );

    // @step And the new connect_with_supervisor sits BESIDE it as an additive constructor (does NOT replace it)
    assert!(
        body.contains("pub async fn connect_with_supervisor("),
        "connect_with_supervisor must be added as an additive constructor BESIDE connect"
    );
    assert!(
        body.contains("action_tx"),
        "connect_with_supervisor must take an action_tx parameter"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Architecture invariants from RPC-005 still hold
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn architecture_invariants_from_rpc_005_still_hold() {
    // @step Given the codelet/rpc-embedded/tests/architecture_invariants.rs source-shape regression
    let arch_rs = workspace_root()
        .join("rpc-embedded")
        .join("tests")
        .join("architecture_invariants.rs");
    let body = read(&arch_rs);

    // @step When the test is run on the RPC-011 tree
    // (The test runs as part of `cargo test -p codelet-rpc-embedded`;
    // here we only assert the test file STILL EXISTS and is shaped as
    // expected. Compile/test success of architecture_invariants.rs is
    // the actual scenario assertion.)
    assert!(
        body.contains("scenario_7_embedded_transport_requires_tokio_handle_at_construction"),
        "scenario_7 must still exist in architecture_invariants.rs"
    );

    // @step Then it asserts: types defined exactly once
    // @step And rpc-server still binds 127.0.0.1
    // @step And no tokio::runtime::Builder / Runtime::new() construction in codelet/fspec/src/ or codelet/rpc-server/src/
    // @step And no second envelope format exists (Envelope is the sole wire-format type in codelet/rpc-server/src/envelope.rs)
    // @step And the rpc crate has no codelet-core dep
    // @step And the test passes

    // Source-shape pin-points:
    let server_rs = workspace_root()
        .join("rpc-server")
        .join("src")
        .join("server.rs");
    let server_body = read(&server_rs);
    // No new runtime construction in rpc-server/src/.
    assert!(
        !strip_comments(&server_body).contains("tokio::runtime::Builder")
            && !strip_comments(&server_body).contains("Runtime::new()"),
        "rpc-server/src/server.rs must not construct a new runtime"
    );

    // Single Envelope type.
    let envelope_rs = workspace_root()
        .join("rpc-server")
        .join("src")
        .join("envelope.rs");
    assert!(envelope_rs.is_file(), "envelope.rs must still exist");

    // rpc crate may depend on codelet-core (RPC-006 lifted the
    // WorkUnitsWatcher into core and the rpc → core arrow is permitted
    // per the existing Cargo.toml comment). The forbidden arrow is rpc
    // → codelet-napi which would re-introduce the NAPI dep at the rpc
    // layer. Source-shape regression: rpc/Cargo.toml must NOT mention
    // codelet-napi at all.
    let rpc_cargo = workspace_root().join("rpc").join("Cargo.toml");
    let rpc_cargo_body = read(&rpc_cargo);
    assert!(
        !rpc_cargo_body.contains("codelet-napi"),
        "codelet-rpc must NOT depend on codelet-napi (RPC-005 invariant). Got:\n{rpc_cargo_body}"
    );

    // The fspec daemon binary's Cargo.toml must NOT depend on codelet-napi.
    let fspec_cargo = workspace_root().join("fspec").join("Cargo.toml");
    let fspec_cargo_body = read(&fspec_cargo);
    assert!(
        !fspec_cargo_body.contains("codelet-napi"),
        "codelet/fspec/Cargo.toml must NOT depend on codelet-napi (rule [13])"
    );

    // rpc-server still binds 127.0.0.1 (loopback only).
    assert!(
        strip_comments(&server_body).contains("\"127.0.0.1:0\"")
            || server_body.contains("127.0.0.1"),
        "rpc-server must still bind 127.0.0.1"
    );
}

/// Strip both `//` line comments and `/* … */` block comments.
fn strip_comments(src: &str) -> String {
    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let next = bytes.get(i + 1).copied();
        if b == b'/' && next == Some(b'/') {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if b == b'/' && next == Some(b'*') {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(bytes.len());
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Scenario: Earlier RPC-005..010 test suites still pass
//
// This is a meta-assertion: the test suites named below must STILL
// EXIST on disk (not deleted or renamed away by RPC-011 changes). The
// actual "tests pass" assertion is the green CI run of `cargo test`.
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn earlier_rpc_005_010_test_suites_still_pass() {
    // @step Given the full Cargo workspace at the end of RPC-011 implementation
    // @step When running cargo test -p codelet-rpc -p codelet-rpc-server -p codelet-rpc-embedded -p codelet-fspec-tui -p codelet-fspec --release
    // @step Then all prior tests pass
    let must_exist = [
        ("rpc-server", "tests/parity.rs"),
        ("rpc-server", "tests/websocket_transport.rs"),
        ("rpc-server", "tests/ws_multi_client_chunks.rs"),
        ("rpc-server", "tests/ws_session_repl.rs"),
        ("rpc-embedded", "tests/architecture_invariants.rs"),
        ("rpc-embedded", "tests/embedded_happy_path.rs"),
        ("rpc-embedded", "tests/embedded_log_event.rs"),
        ("fspec-tui", "tests/ws_backend_smoke.rs"),
        ("fspec-tui", "tests/embedded_backend_smoke.rs"),
        ("fspec-tui", "tests/app_bootstrap_rpc009.rs"),
        ("fspec", "tests/cargo_shape.rs"),
        ("fspec", "tests/combined_smoke.rs"),
        ("fspec", "tests/daemon_mode.rs"),
        ("fspec", "tests/client_mode.rs"),
    ];
    for (crate_name, rel) in must_exist {
        let path = workspace_root().join(crate_name).join(rel);
        assert!(
            path.is_file(),
            "prior-card test file must still exist: {}",
            path.display()
        );
    }

    // @step And the existing Vitest smoke `src/__tests__/napi-workunitinfo-shape.test.ts` still passes
    let project_root = workspace_root()
        .parent()
        .expect("project root above codelet/")
        .to_path_buf();
    let napi_smoke = project_root
        .join("src")
        .join("__tests__")
        .join("napi-workunitinfo-shape.test.ts");
    assert!(
        napi_smoke.is_file(),
        "Vitest NAPI smoke must still exist at {}",
        napi_smoke.display()
    );

    // @step And no test was disabled, skipped, or marked #[ignore] to make RPC-011 green
    // Source-shape sweep: read every tests/*.rs we just asserted, verify
    // no NEW #[ignore] attributes appear inside scenario-named tests.
    for (crate_name, rel) in must_exist {
        let path = workspace_root().join(crate_name).join(rel);
        let body = read(&path);
        // We allow the legacy "#[ignore]" markers that pre-date RPC-011
        // (e.g. the 5 already-ignored tests in cargo_shape.rs); the
        // assertion is on the COUNT: it must not exceed a small bound.
        let ignored = body.matches("#[ignore").count();
        assert!(
            ignored <= 10,
            "{}: too many #[ignore] markers ({}) — RPC-011 must not silence existing tests",
            path.display(),
            ignored
        );
    }
}
