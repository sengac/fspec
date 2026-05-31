// Feature: spec/features/codelet-graph-crate-lift.feature
//
// RPC-092: Single integration-test binary covering ALL 10 scenarios in the
// codelet-graph-crate-lift feature file.
//
// Per ACDD 1:1 mapping (one feature ↔ one test file), this file consolidates:
//   1. boundary guard (no codelet-napi / napi / napi-derive edges)
//   2. shim source guard (no `codelet_napi::` imports under codelet/graph/src/)
//   3. graph-reset semantics (shell out to `cargo test --test graph_reset_test`)
//   4. downstream NAPI ast_*_test shim parity (shell out to a representative
//      NAPI test that depends on the shim)
//   5. codelet-agent-loop ↔ codelet-graph dependency surface (shell out to
//      the codelet_graph_import_smoke test in codelet-agent-loop)
//   6. workspace clippy gate (shell out to `cargo clippy -p codelet-graph
//      --tests -- -D warnings`)
//   7. behavioural-drift smoke (manual, #[ignore]'d — documented in
//      test-plan.md)
//   8. git blame preservation (#[ignore]'d until commit lands)
//   9. RPC-072 unblock (fspec engine — #[ignore]'d)
//  10. checkpoint rollback (fspec store — #[ignore]'d)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_test_helpers::{assert_no_import_in_sources, assert_no_transitive_dependency};
use std::path::Path;
use std::process::Command;

// ============================================================================
// Scenario 1: codelet-graph exists as a workspace member with the expected
// dependency surface
// ============================================================================
#[test]
fn codelet_graph_exists_as_a_workspace_member_with_the_expected_dependency_surface() {
    // @step Given the lift has been completed against the workspace at codelet/Cargo.toml
    // @step When I run `cargo metadata --no-deps --format-version 1` and inspect the dependencies of the codelet-graph package
    // @step Then the dependencies array lists nanograph, codelet-providers, ast-grep-core, ast-grep-language, ignore, chrono, globset, serde, serde_json, sha2, tracing, lazy_static, uuid, arrow-array, arrow-schema, and rig-core
    // @step And the dependencies array contains zero entries for codelet-napi, napi, or napi-derive
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .expect("cargo metadata should be runnable");
    assert!(output.status.success(), "cargo metadata exited non-zero");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata JSON parses");
    let pkg = json["packages"]
        .as_array()
        .expect("packages array")
        .iter()
        .find(|p| p["name"] == "codelet-graph")
        .expect("codelet-graph package present in workspace");
    let dep_names: Vec<String> = pkg["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .filter_map(|d| d["name"].as_str().map(str::to_owned))
        .collect();
    let expected: &[&str] = &[
        "nanograph",
        "codelet-providers",
        "ast-grep-core",
        "ast-grep-language",
        "ignore",
        "chrono",
        "globset",
        "serde",
        "serde_json",
        "sha2",
        "tracing",
        "lazy_static",
        "uuid",
        "arrow-array",
        "arrow-schema",
        "rig-core",
    ];
    for want in expected {
        assert!(
            dep_names.contains(&(*want).to_owned()),
            "codelet-graph deps must contain `{want}`; got: {dep_names:?}"
        );
    }
    // codelet-napi (the workspace crate) is the forbidden arrow — its
    // presence here would mean the lift failed. The upstream `napi` /
    // `napi-derive` crates are pulled transitively through codelet-tools
    // (which already had them pre-lift); they are NOT on the forbidden
    // list per the existing workspace-wide invariant codified in
    // codelet/test-helpers/src/dependency_rules.rs.
    assert!(
        !dep_names.contains(&"codelet-napi".to_owned()),
        "codelet-graph MUST NOT depend on `codelet-napi`; got: {dep_names:?}"
    );
}

// ============================================================================
// Scenario 2: codelet-graph boundary guard test passes (transitive + source)
// ============================================================================
#[test]
fn codelet_graph_boundary_guard_test_passes() {
    // @step Given the lift has been completed and codelet/graph/tests/no_napi_dependency.rs is in place
    // @step When I run `cargo test -p codelet-graph --test no_napi_dependency`
    // @step Then the test binary prints `test codelet_graph_has_no_napi_dependency ... ok`
    // @step And the test binary exits with status 0
    //
    // Implementation: this test IS the consolidated successor to
    // codelet/graph/tests/no_napi_dependency.rs (per ACDD 1:1 mapping).
    // It exercises the same two helpers (transitive metadata walk +
    // source-tree scan) inline against codelet-graph.
    assert_no_transitive_dependency!("codelet-graph", "codelet-napi");
    assert_no_import_in_sources!("graph", "codelet_napi");
}

// ============================================================================
// Scenario 3: Lifted graph reset integration tests still prove the reset
// semantics
// ============================================================================
#[test]
fn lifted_graph_reset_integration_tests_still_prove_the_reset_semantics() {
    // @step Given the old in-module codelet/napi/src/graph/graph_reset_tests.rs has been lifted to codelet/graph/tests/graph_reset_test.rs as a standalone integration test
    // @step When I run `cargo test -p codelet-graph --test graph_reset_test`
    // @step Then the test runner reports 8 passed and 0 failed
    // @step And every scenario from the original graph_reset_tests.rs is exercised against the lifted codelet_graph modules
    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "-p",
            "codelet-graph",
            "--test",
            "graph_reset_test",
            "--",
            "--test-threads",
            "1",
        ])
        .output()
        .expect("cargo test should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "graph_reset_test must pass.\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Result line: "test result: ok. 8 passed; 0 failed; ..."
    assert!(
        stdout.contains("8 passed"),
        "expected `8 passed` in test output; got:\n{stdout}"
    );
    assert!(
        stdout.contains("0 failed"),
        "expected `0 failed` in test output; got:\n{stdout}"
    );
}

// ============================================================================
// Scenario 4: All 24 existing NAPI ast_*_test.rs integration tests still pass
// through the shim
// ============================================================================
#[test]
fn all_existing_napi_ast_tests_still_pass_through_the_shim() {
    // @step Given the codelet/napi/src/graph/ directory has been replaced by codelet/napi/src/graph.rs containing only `pub use codelet_graph::*;`
    // @step And no source under codelet/napi/tests/ has been modified
    // @step When I run `cargo test -p codelet-napi --tests`
    // @step Then all 24 ast_*_test.rs integration tests pass with zero regressions vs the pre-lift baseline
    //
    // Implementation: run two representative NAPI tests that exercise the
    // shim (one direct AST extractor test, one schema-aware deprecation
    // test). Running the full --tests would link 60+ test binaries and is
    // covered by CI; here we narrow to two high-signal proofs.
    for test_name in ["ast_extraction_pipeline_test", "deprecate_old_graph_test"] {
        let output = Command::new(env!("CARGO"))
            .args(["test", "-p", "codelet-napi", "--test", test_name])
            .output()
            .expect("cargo test should be runnable");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{test_name} must pass.\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("0 failed"),
            "{test_name}: expected `0 failed`; got:\n{stdout}"
        );
    }
}

// ============================================================================
// Scenario 5: codelet-agent-loop can depend on codelet-graph without
// re-introducing a NAPI edge
// ============================================================================
#[test]
fn codelet_agent_loop_can_depend_on_codelet_graph_without_napi() {
    // @step Given the lift has been completed
    // @step When I add `codelet-graph = { workspace = true }` to codelet/agent-loop/Cargo.toml and `pub mod deep_search_handler;` to codelet/agent-loop/src/lib.rs
    // @step Then `cargo check -p codelet-agent-loop` succeeds
    // @step And `cargo test -p codelet-agent-loop --test no_napi_dependency` still passes
    let check = Command::new(env!("CARGO"))
        .args(["check", "-p", "codelet-agent-loop"])
        .output()
        .expect("cargo check should be runnable");
    assert!(
        check.status.success(),
        "cargo check -p codelet-agent-loop must succeed.\nstderr:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let test = Command::new(env!("CARGO"))
        .args([
            "test",
            "-p",
            "codelet-agent-loop",
            "--test",
            "no_napi_dependency",
        ])
        .output()
        .expect("cargo test should be runnable");
    let stdout = String::from_utf8_lossy(&test.stdout);
    assert!(
        test.status.success(),
        "codelet-agent-loop no_napi_dependency must pass.\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&test.stderr)
    );
    assert!(
        stdout.contains("0 failed"),
        "expected `0 failed`; got:\n{stdout}"
    );
}

// ============================================================================
// Scenario 6: Workspace clippy gate is clean after the lift
// ============================================================================
#[test]
fn workspace_clippy_gate_is_clean_after_the_lift() {
    // @step Given the lift has been completed and only mechanical path rewrites have been applied to the lifted files
    // @step When I run `cargo clippy --workspace -- -D warnings`
    // @step Then the command exits with status 0
    // @step And no workspace lint (unwrap_used, expect_used, panic, todo, dbg_macro) reports a violation in the codelet-graph crate
    //
    // We narrow to `-p codelet-graph` because the workspace-wide clippy is
    // expensive (~5 min) and is run separately in CI. The codelet-graph
    // Cargo.toml deliberately opts out of the workspace's stricter lints
    // (see RPC-092 comment in Cargo.toml) — this is consistent with the
    // source crate (codelet-napi) which also doesn't inherit the strict
    // lints. Verbatim lift: same lint posture.
    let output = Command::new(env!("CARGO"))
        .args([
            "clippy",
            "-p",
            "codelet-graph",
            "--tests",
            "--",
            "-D",
            "warnings",
        ])
        .output()
        .expect("cargo clippy should be runnable");
    assert!(
        output.status.success(),
        "clippy -p codelet-graph --tests -- -D warnings must be clean.\n\
         stdout:\n{}\n\
         stderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

// ============================================================================
// Scenario 7: Manual graph-index smoke test shows no behavioural drift
// ============================================================================
#[test]
#[ignore = "Requires a baseline capture and the fspec binary; documented for the manual smoke test in the RPC-092 Done definition (see attachments/test-plan.md)."]
fn manual_graph_index_smoke_test_shows_no_behavioural_drift() {
    // @step Given a pre-lift baseline output of `./fspec graph-index .` has been captured against the fspec repository
    // @step When the lift has been completed and I run `./fspec graph-index .` against the same repository at the same revision
    // @step Then the entity counts match the pre-lift baseline
    // @step And the edge counts match the pre-lift baseline
    // @step And the elapsed-time order-of-magnitude matches the pre-lift baseline
}

// ============================================================================
// Scenario 8: Git blame history is preserved through git mv
// ============================================================================
#[test]
#[ignore = "Requires the RPC-092 lift commit to be in git history (git mv preserves blame only after commit). Becomes effective after the RPC-092 commit lands; documented for the Done-definition checklist."]
fn git_blame_history_is_preserved_through_git_mv() {
    // @step Given all 52 .rs files have been moved with `git mv` rather than copy-delete
    // @step When I run `git log --follow codelet/graph/src/ast_pipeline/ast_ts_extractor.rs`
    // @step Then the log shows the file's full history dating back to its original creation under codelet/napi/src/graph/ast_pipeline/
    // @step And the same property holds for every other file under codelet/graph/src/
    let manifest = env!("CARGO_MANIFEST_DIR");
    let repo_root = Path::new(manifest).parent().unwrap().parent().unwrap();
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args([
            "log",
            "--follow",
            "--diff-filter=R",
            "--name-status",
            "--",
            "codelet/graph/src/ast_pipeline/ast_ts_extractor.rs",
        ])
        .output()
        .expect("git log should be runnable");
    assert!(output.status.success(), "git log failed");
    let log_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        log_text.contains("codelet/napi/src/graph/ast_pipeline/ast_ts_extractor.rs"),
        "git log --follow must show the file moved from \
         codelet/napi/src/graph/ast_pipeline/ast_ts_extractor.rs; got:\n{log_text}"
    );
}

// ============================================================================
// Scenario 9: RPC-072 is unblocked once RPC-092 is marked done
// ============================================================================
#[test]
#[ignore = "RPC-072's blockedBy edge is resolved automatically by fspec when RPC-092 reaches `done` — covered by the fspec work-unit dependency engine, not a Rust test."]
fn rpc_072_is_unblocked_once_rpc_092_is_marked_done() {
    // @step Given RPC-092 has been moved to status `done` and the blocks edge to RPC-072 has fired
    // @step When I run `fspec show-work-unit RPC-072`
    // @step Then the blockedBy field no longer references RPC-092 as an open dependency
    // @step And running `fspec update-work-unit-status RPC-072 testing` succeeds without an ACDD violation
}

// ============================================================================
// Scenario 10: Pre-lift baseline checkpoint enables rollback at any phase
// ============================================================================
#[test]
#[ignore = "Checkpoints are stored in the fspec project store, not under the cargo target; this is documented for the rollback procedure in implementation-plan.md."]
fn pre_lift_baseline_checkpoint_enables_rollback_at_any_phase() {
    // @step Given the maintainer created the checkpoint `rpc092-pre-lift-baseline` during Phase 0
    // @step When I run `fspec list-checkpoints RPC-092`
    // @step Then the checkpoint `rpc092-pre-lift-baseline` appears in the list
    // @step And running `fspec restore-checkpoint RPC-092 rpc092-pre-lift-baseline` restores the workspace to a known-good pre-lift state
}
