//! Skeleton invariant tests for the `codelet-sessions` crate (RPC-038).
//!
//! Feature: spec/features/codelet-sessions-crate-skeleton.feature
//!
//! These tests codify the static shape of the new crate skeleton by
//! inspecting source files and manifests. They do not exercise any
//! runtime code path. Each `#[test]` corresponds to a single Gherkin
//! scenario in the feature file; the @step comments below map each
//! Gherkin step to the assertion that enforces it.
//!
//! Pattern borrowed from `codelet/rpc-embedded/tests/rpc_006_source_shape.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Workspace root (one level above this crate's manifest dir).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("codelet-sessions manifest dir must have a parent")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn scenario_cargo_workspace_recognises_the_new_codelet_sessions_crate() {
    // @step Given the root `Cargo.toml` lists `codelet/sessions` as a workspace member and `codelet-sessions` as a workspace dependency
    //
    // NOTE: workspace.members paths are relative to the workspace root
    // (`codelet/Cargo.toml`), so the literal in the manifest is "sessions"
    // even though the directory's repo-relative path is `codelet/sessions/`.
    let root_manifest = read(&workspace_root().join("Cargo.toml"));
    assert!(
        root_manifest.contains("\"sessions\""),
        "root Cargo.toml workspace.members must include \"sessions\" (which resolves to codelet/sessions/)"
    );
    assert!(
        root_manifest.contains("codelet-sessions = { path = \"sessions\" }")
            || root_manifest.contains("codelet-sessions = { path = \"./sessions\" }"),
        "root Cargo.toml [workspace.dependencies] must expose codelet-sessions"
    );

    // @step When I run `cargo metadata -p codelet-sessions --format-version 1`
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "cargo metadata failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // @step Then the output JSON includes a package named `codelet-sessions` at version 0.1.0 with manifest path ending in `codelet/sessions/Cargo.toml`
    assert!(
        stdout.contains("\"name\":\"codelet-sessions\""),
        "cargo metadata must list a `codelet-sessions` package"
    );
    assert!(
        stdout.contains("codelet/sessions/Cargo.toml"),
        "cargo metadata manifest_path must point at codelet/sessions/Cargo.toml"
    );
}

#[test]
fn scenario_codelet_sessions_builds_standalone_against_empty_modules() {
    // @step Given the codelet-sessions crate has been scaffolded with empty `background_session` and `session_manager` modules
    let bg = workspace_root()
        .join("sessions")
        .join("src")
        .join("background_session.rs");
    let sm = workspace_root()
        .join("sessions")
        .join("src")
        .join("session_manager.rs");
    assert!(
        bg.exists(),
        "codelet/sessions/src/background_session.rs must exist"
    );
    assert!(
        sm.exists(),
        "codelet/sessions/src/session_manager.rs must exist"
    );

    // @step When I run `cargo build -p codelet-sessions`
    let output = Command::new(env!("CARGO"))
        .args(["build", "-p", "codelet-sessions", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo build must run");

    // @step Then the build completes successfully with no errors
    assert!(
        output.status.success(),
        "cargo build -p codelet-sessions failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi() {
    // @step Given the codelet-sessions crate depends on codelet-rpc-types WITHOUT the `napi` feature
    let sessions_manifest = read(&workspace_root().join("sessions").join("Cargo.toml"));
    assert!(
        sessions_manifest.contains("codelet-rpc-types"),
        "codelet/sessions/Cargo.toml must depend on codelet-rpc-types"
    );
    // Reject any line that wires the `napi` feature on codelet-rpc-types.
    for line in sessions_manifest.lines() {
        let trimmed = line.trim();
        if trimmed.contains("codelet-rpc-types") && trimmed.contains("\"napi\"") {
            panic!(
                "codelet/sessions/Cargo.toml must NOT enable the napi feature on codelet-rpc-types: {trimmed}"
            );
        }
    }

    // @step When I run `cargo metadata -p codelet-sessions --format-version 1` and inspect the transitive package set
    //
    // We use a workspace-wide `cargo metadata` (no --filter-platform —
    // that flag wants a full target triple, not just an arch) and then
    // walk the resolve graph rooted at codelet-sessions to determine
    // the closure of its transitive dependencies.
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "cargo metadata failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Build the inverted "what packages does codelet-sessions transitively depend on?"
    // view by parsing the resolve.nodes list. We avoid pulling a JSON parser dep
    // by doing a coarse-grained substring scan, which is sufficient for the
    // forbidden-arrow check (we are asserting the *absence* of a substring).
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("metadata JSON must parse");
    let resolve = json
        .get("resolve")
        .expect("cargo metadata must include a resolve");
    let nodes = resolve
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("resolve.nodes must be an array");

    // Find the codelet-sessions node id
    let packages = json
        .get("packages")
        .and_then(|v| v.as_array())
        .expect("packages must be an array");
    let sessions_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("codelet-sessions"))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .expect("codelet-sessions package must exist in metadata")
        .to_string();

    // Closure: collect transitive deps via BFS over resolve.nodes
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stack: Vec<String> = vec![sessions_id];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|i| i.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for d in deps {
                        if let Some(s) = d.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
                break;
            }
        }
    }

    // Map node ids back to package names by looking each up in packages.
    let mut transitive_names: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for id in &seen {
        if let Some(pkg) = packages
            .iter()
            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                transitive_names.insert(name.to_string());
            }
        }
    }

    // @step Then no package named `codelet-napi` appears anywhere in the transitive dependency graph
    assert!(
        !transitive_names.contains("codelet-napi"),
        "codelet-sessions must not transitively depend on codelet-napi. Transitive set: {transitive_names:?}"
    );

    // @step And the only NAPI-related crates that appear are the third-party `napi` / `napi-derive` bindings pulled in by an existing inbound dependency outside the scope of this card — never the local `codelet-napi` crate
    //
    // The scope of RPC-038 is the local `codelet-napi` crate only. The
    // third-party `napi` / `napi-derive` bindings on crates.io are
    // currently a transitive dep of `codelet-tools` (see
    // codelet/tools/Cargo.toml lines 80-81). Removing them is a separate
    // pre-existing concern handled by a future card. This test
    // therefore documents the invariant: if `napi` ever appears, it must
    // come from a non-local source, never via `codelet-napi`.
    if transitive_names.contains("napi") || transitive_names.contains("napi-derive") {
        // OK as long as codelet-napi itself is absent (asserted above).
        // Tracking note for future readers: see codelet/tools/Cargo.toml.
        eprintln!(
            "[RPC-038] note: transitive napi crate is present via a non-local inbound dep; this is documented as out-of-scope (see codelet/tools/Cargo.toml). transitive_names contains: napi={}, napi-derive={}",
            transitive_names.contains("napi"),
            transitive_names.contains("napi-derive")
        );
    }
}

#[test]
fn scenario_smoke_test_runs_and_passes() {
    // @step Given `codelet/sessions/tests/smoke.rs` contains a `crate_compiles` test
    let smoke_path = workspace_root()
        .join("sessions")
        .join("tests")
        .join("smoke.rs");
    let smoke = read(&smoke_path);
    assert!(
        smoke.contains("fn crate_compiles"),
        "codelet/sessions/tests/smoke.rs must define `fn crate_compiles`"
    );
    assert!(
        smoke.contains("#[test]"),
        "codelet/sessions/tests/smoke.rs must mark `crate_compiles` with #[test]"
    );

    // @step When I run `cargo test -p codelet-sessions`
    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "-p",
            "codelet-sessions",
            "--test",
            "smoke",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo test must run");

    // @step Then the `crate_compiles` test is discovered and passes with status `ok`
    assert!(
        output.status.success(),
        "cargo test -p codelet-sessions --test smoke failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("crate_compiles") && combined.contains("ok"),
        "smoke test output must report `crate_compiles ... ok`. Got: {combined}"
    );
}

#[test]
fn scenario_lib_rs_declares_placeholder_modules_for_later_rpc_cards() {
    // @step Given the codelet-sessions crate skeleton has been created
    let lib_path = workspace_root().join("sessions").join("src").join("lib.rs");
    assert!(lib_path.exists(), "codelet/sessions/src/lib.rs must exist");

    // @step When I read `codelet/sessions/src/lib.rs`
    let lib = read(&lib_path);

    // @step Then it declares `pub mod background_session;`
    assert!(
        lib.contains("pub mod background_session;"),
        "codelet/sessions/src/lib.rs must declare `pub mod background_session;`"
    );

    // @step And it declares `pub mod session_manager;`
    assert!(
        lib.contains("pub mod session_manager;"),
        "codelet/sessions/src/lib.rs must declare `pub mod session_manager;`"
    );

    // @step And both module files exist with placeholder doc comments naming the cards that will populate them
    let bg_path = workspace_root()
        .join("sessions")
        .join("src")
        .join("background_session.rs");
    let sm_path = workspace_root()
        .join("sessions")
        .join("src")
        .join("session_manager.rs");
    let bg = read(&bg_path);
    let sm = read(&sm_path);
    assert!(
        bg.contains("RPC-039"),
        "background_session.rs placeholder must reference RPC-039"
    );
    assert!(
        sm.contains("RPC-040"),
        "session_manager.rs placeholder must reference RPC-040"
    );
    // Both placeholders use the `//! Placeholder.` doc-comment idiom.
    assert!(
        bg.contains("//!"),
        "background_session.rs must start with a //! doc comment"
    );
    assert!(
        sm.contains("//!"),
        "session_manager.rs must start with a //! doc comment"
    );
}

#[test]
fn scenario_workspace_lints_are_inherited_and_clippy_passes() {
    // @step Given `codelet/sessions/Cargo.toml` declares `[lints]` with `workspace = true`
    let manifest = read(&workspace_root().join("sessions").join("Cargo.toml"));
    let has_workspace_lints = manifest.lines().any(|l| {
        let t = l.trim();
        t.starts_with("workspace") && t.contains("= true")
    });
    assert!(
        manifest.contains("[lints]"),
        "codelet/sessions/Cargo.toml must declare a [lints] section"
    );
    assert!(
        has_workspace_lints,
        "codelet/sessions/Cargo.toml [lints] must inherit `workspace = true`"
    );

    // @step When I run `cargo clippy -p codelet-sessions --all-targets -- -D warnings`
    let output = Command::new(env!("CARGO"))
        .args([
            "clippy",
            "-p",
            "codelet-sessions",
            "--all-targets",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .args(["--", "-D", "warnings"])
        .output()
        .expect("cargo clippy must run");

    // @step Then clippy completes successfully with no warnings or errors
    assert!(
        output.status.success(),
        "cargo clippy -p codelet-sessions failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}
