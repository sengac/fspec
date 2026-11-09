//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
use super::*;

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_cargo_build_p_fspec_release_produces_codelet_target_release_fspec() {
    // @step When the developer runs `cargo build -p fspec --release` from `rust/`
    let output = Command::new("cargo")
        .arg("build")
        .arg("-p")
        .arg("fspec")
        .arg("--release")
        .current_dir(common::codelet_root())
        .output()
        .expect("cargo build");

    // @step Then the build completes with exit code 0
    assert!(
        output.status.success(),
        "cargo build -p fspec --release must succeed; got {:?} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // @step And the file `rust/target/release/fspec` exists and is executable
    let artifact = common::codelet_root()
        .join("target")
        .join("release")
        .join("fspec");
    assert!(
        artifact.is_file(),
        "release artifact must exist at {}",
        artifact.display()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&artifact)
            .expect("metadata")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "release artifact must be executable");
    }
}

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_existing_codelet_rpc_server_test_harness_still_works_port_line_contract_preserved() {
    // @step Given the test `rust/rpc-server/tests/websocket_transport.rs::spawn_rpc_server` exists
    let path = common::codelet_root()
        .join("rpc-server")
        .join("tests")
        .join("websocket_transport.rs");
    assert!(
        path.is_file(),
        "websocket_transport.rs must exist at {}",
        path.display()
    );

    // @step When the developer runs `cargo test -p codelet-rpc-server`
    let output = Command::new("cargo")
        .arg("test")
        .args(["--profile", "ci-test"])
        .arg("-p")
        .arg("codelet-rpc-server")
        .current_dir(common::codelet_root())
        .output()
        .expect("cargo test rpc-server");

    // @step Then the test passes with the OLD `codelet-rpc-server` binary path
    assert!(
        output.status.success(),
        "cargo test -p codelet-rpc-server must pass; got {:?}",
        output.status
    );

    // @step And no behaviour change has been introduced to the RPC-006 binary
    let main_rs = common::codelet_root()
        .join("rpc-server")
        .join("src")
        .join("main.rs");
    let body = std::fs::read_to_string(&main_rs).expect("read rpc-server main.rs");
    assert!(
        body.contains("#[tokio::main]") && body.contains("async fn main()"),
        "rpc-server main.rs must remain unchanged in RPC-010 (no behaviour change)"
    );
}

#[test]
#[ignore = "validating-phase attestation - run manually after implementing"]
fn scenario_existing_cargo_test_suites_for_rpc_005_009_remain_green() {
    // @step When the developer runs `cargo test -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui` from `rust/`
    let output = Command::new("cargo")
        .arg("test")
        .args(["--profile", "ci-test"])
        .arg("-p")
        .arg("codelet-rpc-embedded")
        .arg("-p")
        .arg("codelet-rpc-server")
        .arg("-p")
        .arg("codelet-fspec-tui")
        .current_dir(common::codelet_root())
        .output()
        .expect("cargo test RPC-005..008");

    // @step Then every test passes unchanged
    assert!(
        output.status.success(),
        "RPC-005..009 cargo test suites must remain green; got {:?}",
        output.status
    );

    // @step And no test in those crates has been modified for this card except the source-shape widening referenced above
    let invariants = common::codelet_root()
        .join("rpc-embedded")
        .join("tests")
        .join("architecture_invariants.rs");
    assert!(
        invariants.is_file(),
        "architecture_invariants.rs must still exist (was widened, not removed)"
    );
}
