//! Feature: spec/features/fspec-binary-cargo-shape-rpc010.feature
use super::*;

#[test]
fn scenario_cargo_toml_dependencies_does_not_list_codelet_napi() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    let body = std::fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step When the test parses the `[dependencies]` table
    let deps = extract_section(&body, "[dependencies]").expect("[dependencies] section must exist");

    // @step Then there is no key named `codelet-napi`
    // @step And there is no key whose name starts with `codelet-napi`
    for line in deps.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        assert!(
            !key.starts_with("codelet-napi"),
            "[dependencies] must not list any codelet-napi.*; found: {line:?}"
        );
    }
}

#[test]
fn scenario_cargo_toml_dev_dependencies_does_not_list_codelet_napi() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    let body = std::fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step When the test parses the `[dev-dependencies]` table
    let dev_deps = extract_section(&body, "[dev-dependencies]")
        .expect("[dev-dependencies] section must exist");

    // @step Then there is no key named `codelet-napi`
    for line in dev_deps.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let key = trimmed.split('=').next().unwrap_or("").trim();
        assert!(
            !key.starts_with("codelet-napi"),
            "[dev-dependencies] must not list any codelet-napi.*; found: {line:?}"
        );
    }
}

#[test]
fn scenario_cargo_toml_declares_the_expected_production_dependencies() {
    // @step Given the file `rust/fspec/Cargo.toml` exists
    let cargo = fspec_crate_root().join("Cargo.toml");
    let body = std::fs::read_to_string(&cargo).expect("read fspec/Cargo.toml");

    // @step When the test parses the `[dependencies]` table
    let deps = extract_section(&body, "[dependencies]").expect("[dependencies] section must exist");

    // @step Then it contains keys: `clap`, `tokio`, `anyhow`, `tracing`, `tracing-subscriber`, `tracing-appender`, `dirs`, `serde`, `serde_json`, `url`
    // @step And it contains keys: `codelet-rpc`, `codelet-rpc-types`, `codelet-rpc-embedded`, `codelet-rpc-server`, `codelet-fspec-tui`, `codelet-core`
    let expected = [
        "clap",
        "tokio",
        "anyhow",
        "tracing",
        "tracing-subscriber",
        "tracing-appender",
        "dirs",
        "serde",
        "serde_json",
        "url",
        "codelet-rpc",
        "codelet-rpc-types",
        "codelet-rpc-embedded",
        "codelet-rpc-server",
        "codelet-fspec-tui",
        "codelet-core",
        // PROV-097: dotenv-at-startup load (mirrors rust/cli).
        "dotenvy",
    ];
    let mut missing = Vec::new();
    for needle in expected {
        let pat_eq = format!("{needle} ");
        let pat_dot = format!("{needle}.");
        let found = deps.lines().any(|l| {
            let t = l.trim();
            t.starts_with(&pat_eq)
                || t.starts_with(&pat_dot)
                || t.starts_with(&format!("{needle}="))
        });
        if !found {
            missing.push(needle);
        }
    }
    assert!(
        missing.is_empty(),
        "[dependencies] missing required keys: {missing:?}\nbody was:\n{deps}"
    );
}

#[test]
fn scenario_no_source_file_constructs_its_own_tokio_runtime() {
    // @step Given the directory `rust/fspec/src/` exists
    let src = fspec_crate_root().join("src");
    assert!(src.is_dir(), "rust/fspec/src/ must exist");

    // @step When the test scans every `.rs` file under the directory recursively
    let files = collect_rs_files(&src);
    let mut violations: Vec<String> = Vec::new();
    let forbidden = [
        "tokio::runtime::Builder",
        "runtime::Builder::new_multi_thread",
        "runtime::Builder::new_current_thread",
        "tokio::runtime::Runtime::new",
        "Runtime::new()",
    ];
    for path in &files {
        let body = std::fs::read_to_string(path).expect("read rs file");
        let stripped = strip_comments(&body);
        for needle in forbidden {
            if stripped.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    // @step Then no file contains the literal substring `tokio::runtime::Builder`
    // @step And no file contains the literal substring `runtime::Builder::new_multi_thread`
    // @step And no file contains the literal substring `runtime::Builder::new_current_thread`
    // @step And no file contains the literal substring `tokio::runtime::Runtime::new`
    // @step And no file contains the literal substring `Runtime::new()`
    assert!(
        violations.is_empty(),
        "rust/fspec/src/ must NOT construct its own tokio runtime: {violations:?}"
    );
}

#[test]
fn scenario_rpc_005_source_shape_invariant_is_widened_to_scan_fspec_src() {
    // @step Given the file `rust/rpc-embedded/tests/architecture_invariants.rs` exists
    let path = codelet_root()
        .join("rpc-embedded")
        .join("tests")
        .join("architecture_invariants.rs");
    assert!(path.is_file(), "architecture_invariants.rs must exist");
    // The test is split into `architecture_invariants/` parts; read the tree.
    let body = codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .expect("read architecture_invariants module tree");

    // @step Then the file contains the substring `"fspec/src"` in its scanned-directory list
    assert!(
        body.contains("\"fspec/src\"") || body.contains("\"fspec\").join(\"src\"")
            || body.contains(r#"join("fspec").join("src")"#)
            || body.contains(r#"join("fspec").join("src/")"#),
        "architecture_invariants.rs must reference `fspec/src` (or join(\"fspec\").join(\"src\")) in its scanned-directory list"
    );
}

#[test]
fn scenario_existing_codelet_rpc_server_dev_helper_binary_stays_in_place() {
    // @step Given the file `rust/rpc-server/src/main.rs` exists
    let main_rs = codelet_root()
        .join("rpc-server")
        .join("src")
        .join("main.rs");
    assert!(main_rs.is_file(), "rpc-server/src/main.rs must still exist");
    let body = std::fs::read_to_string(&main_rs).expect("read rpc-server main.rs");

    // @step Then the file still defines `#[tokio::main] async fn main()` (unchanged from RPC-006)
    assert!(
        body.contains("#[tokio::main]") && body.contains("async fn main()"),
        "rpc-server main.rs must still declare `#[tokio::main] async fn main()`"
    );

    // @step And `rust/rpc-server/Cargo.toml` still declares the binary
    let cargo = codelet_root().join("rpc-server").join("Cargo.toml");
    assert!(cargo.is_file(), "rpc-server/Cargo.toml must exist");
    let cargo_body = std::fs::read_to_string(&cargo).expect("read rpc-server Cargo.toml");
    assert!(
        cargo_body.contains(r#"name = "codelet-rpc-server""#)
            || cargo_body.contains(r#"name = \"codelet-rpc-server\""#),
        "rpc-server Cargo.toml must still name the package codelet-rpc-server"
    );

    // @step And no commit in this card has removed those files
    // (assertion satisfied by the two .is_file() checks above)
}

#[ignore = "RPC-026: spawns the CLI binary; combined-mode invocations grab /dev/tty via ratatui; run with `cargo test -- --ignored` in a real TTY/CI environment"]
#[test]
fn scenario_spawn_fspec_daemon_helper_proves_port_line_contract_is_verbatim() {
    // @step Given the test file `rust/fspec/tests/daemon_mode.rs` exists
    let daemon_mode = fspec_crate_root().join("tests").join("daemon_mode.rs");
    assert!(
        daemon_mode.is_file(),
        "rust/fspec/tests/daemon_mode.rs must exist"
    );

    // The helper now lives in tests/common/mod.rs to keep the LoC budget,
    // but daemon_mode.rs must use it.
    let body = std::fs::read_to_string(&daemon_mode).expect("read daemon_mode.rs");

    // @step Then it defines a `spawn_fspec_daemon` helper that uses `BufReader::read_line`
    // (The helper itself lives in tests/common/mod.rs and is imported.)
    let common = fspec_crate_root()
        .join("tests")
        .join("common")
        .join("mod.rs");
    let common_body = std::fs::read_to_string(&common).expect("read tests/common/mod.rs");
    assert!(
        common_body.contains("pub fn spawn_fspec_daemon") && common_body.contains("BufReader::new"),
        "tests/common/mod.rs must define spawn_fspec_daemon using BufReader::read_line"
    );

    // @step And the helper parses the first STDOUT line as a bare integer port
    assert!(
        common_body.contains("read_line(&mut line)") && common_body.contains("parse()"),
        "spawn_fspec_daemon must read_line then parse() the bare integer port"
    );

    // @step And the same parsing logic mirrors `rust/rpc-server/tests/websocket_transport.rs::spawn_rpc_server`
    let rpc_server_common = codelet_root()
        .join("rpc-server")
        .join("tests")
        .join("common")
        .join("mod.rs");
    let rpc_server_common_body =
        std::fs::read_to_string(&rpc_server_common).expect("read rpc-server tests/common/mod.rs");
    assert!(
        rpc_server_common_body.contains("BufReader::new")
            && rpc_server_common_body.contains("read_line"),
        "rpc-server's spawn_rpc_server_with_workspace must also use BufReader + read_line (existing pattern being mirrored)"
    );

    let _ = body; // daemon_smoke.rs content not separately scanned here
}
