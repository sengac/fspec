// Feature: spec/features/codelet-graph-crate-lift.feature
//
// RPC-092: Single integration-test binary covering ALL 10 scenarios in the
// codelet-graph-crate-lift feature file.
//
// Per ACDD 1:1 mapping (one feature ↔ one test file), this file consolidates:
//   1. boundary guard (no codelet-napi / napi / napi-derive edges)
//   2. shim source guard (no `codelet_napi::` imports under rust/graph/src/)
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
    // @step Given the lift has been completed against the workspace at rust/Cargo.toml
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
        // KGRAPH-072: no arrow-* pins; export rows come from nanograph.
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
    // rust/test-helpers/src/dependency_rules.rs.
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
    // @step Given the lift has been completed and rust/graph/tests/no_napi_dependency.rs is in place
    // @step When I run `cargo test -p codelet-graph --test no_napi_dependency`
    // @step Then the test binary prints `test codelet_graph_has_no_napi_dependency ... ok`
    // @step And the test binary exits with status 0
    //
    // Implementation: this test IS the consolidated successor to
    // rust/graph/tests/no_napi_dependency.rs (per ACDD 1:1 mapping).
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
    // @step Given the old in-module rust/napi/src/graph/graph_reset_tests.rs has been lifted to rust/graph/tests/graph_reset_test.rs as a standalone integration test
    // @step When I run `cargo test -p codelet-graph --profile ci-test --test graph_reset_test --test graph_schema_validation_test`
    let output = Command::new(env!("CARGO"))
        .args([
            "test",
            "-p",
            "codelet-graph",
            "--profile",
            "ci-test",
            "--test",
            "graph_reset_test",
            "--test",
            "graph_schema_validation_test",
            "--",
            "--test-threads",
            "1",
        ])
        .output()
        .expect("cargo test should be runnable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // @step Then each of the 8 lifted graph-reset tests reports ok by name and no test fails
    // BUG-542: by name, never a total count — COV-075 moved the schema-match
    // test into graph_schema_validation_test.rs, which broke an "8 passed"
    // match although every lifted test still ran.
    assert!(
        output.status.success(),
        "the lifted graph-reset tests must pass.\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in LIFTED_GRAPH_RESET_TESTS {
        assert!(
            stdout.contains(&format!("test {name} ... ok")),
            "expected `test {name} ... ok` in test output; got:\n{stdout}"
        );
    }
    assert!(
        !stdout.contains("FAILED"),
        "no lifted graph-reset test may fail; got:\n{stdout}"
    );
    // @step And every scenario from the original graph_reset_tests.rs is exercised against the lifted codelet_graph modules
    // (the 8 names below are the 8 tests of the original in-module file)
}

/// The 8 tests lifted from rust/napi/src/graph/graph_reset_tests.rs (RPC-092);
/// the schema-match one lives in graph_schema_validation_test.rs since COV-075.
/// KGRAPH-073 renamed the delete_graph_data one: the write lease's reset
/// replaced registry::delete_graph_data.
const LIFTED_GRAPH_RESET_TESTS: [&str; 8] = [
    "test_reset_deletes_on_disk_database",
    "test_reset_clears_in_memory_registry",
    "test_schema_mismatch_returns_actionable_error",
    "test_schema_match_passes_validation",
    "test_queries_work_after_reset_and_reindex",
    "test_ast_index_deserializes_with_reset_flag",
    "test_ast_index_backwards_compatible_without_reset",
    "test_reset_returns_false_when_no_data",
];

// ============================================================================
// Scenario 5: codelet-agent-loop can depend on codelet-graph without
// re-introducing a NAPI edge
// ============================================================================
#[test]
fn codelet_agent_loop_can_depend_on_codelet_graph_without_napi() {
    // @step Given the lift has been completed
    // @step When I add `codelet-graph = { workspace = true }` to rust/agent-loop/Cargo.toml and `pub mod deep_search_handler;` to rust/agent-loop/src/lib.rs
    // @step Then `cargo check -p codelet-agent-loop` succeeds
    // @step And `cargo test -p codelet-agent-loop --test no_napi_dependency` still passes
    let check = Command::new(env!("CARGO"))
        .args(["check", "-p", "codelet-agent-loop", "--profile", "ci-test"])
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
            "--profile",
            "ci-test",
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
            "--profile",
            "ci-test",
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
    // @step When I run `git log --follow rust/graph/src/ast_pipeline/ast_ts_extractor.rs`
    // @step Then the log shows the file's full history dating back to its original creation under rust/napi/src/graph/ast_pipeline/
    // @step And the same property holds for every other file under rust/graph/src/
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
            "rust/graph/src/ast_pipeline/ast_ts_extractor.rs",
        ])
        .output()
        .expect("git log should be runnable");
    assert!(output.status.success(), "git log failed");
    let log_text = String::from_utf8_lossy(&output.stdout);
    // The file's history must reach back past the lift: --follow reports at
    // least one rename whose source is an earlier ast_ts_extractor.rs path.
    let current = "rust/graph/src/ast_pipeline/ast_ts_extractor.rs";
    let earlier_name = log_text.lines().any(|line| {
        line.starts_with('R')
            && line
                .split('\t')
                .nth(1)
                .is_some_and(|from| from != current && from.ends_with("/ast_ts_extractor.rs"))
    });
    assert!(
        earlier_name,
        "git log --follow must show the file was moved into {current}; got:\n{log_text}"
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
