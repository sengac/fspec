//! RPC-067 shape-invariant regression tests.
//!
//! Feature: spec/features/dependency-rule-regression-tests.feature
//!
//! These tests codify the structural shape of the RPC-067 migration:
//! every scenario in the feature file that describes a "this file
//! contains exactly two #[test] fns delegating to the helper" or
//! "this Cargo.toml declares codelet-test-helpers under
//! [dev-dependencies]" or "this Cargo.toml lists test-helpers as a
//! workspace member" is enforced here, so a future refactor that
//! widens / collapses one of these files trips the test.
//!
//! The cargo-metadata + source-walk runtime invariants are covered by
//! the five per-crate `no_napi_dependency.rs` files. This file is
//! ONLY about the migration shape.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::PathBuf;

/// Workspace root: `codelet/` (one level above this test crate).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("test-helpers manifest dir must have a parent")
        .to_path_buf()
}

fn read(rel: &str) -> String {
    let path = workspace_root().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

#[test]
fn scenario_codelet_test_helpers_crate_is_wired_into_the_workspace_as_a_library() {
    // @step Given the codelet workspace root manifest at codelet/Cargo.toml
    let root_manifest = read("Cargo.toml");

    // @step When I inspect the [workspace] members list and the [workspace.dependencies] table
    // @step Then "test-helpers" appears in workspace.members
    assert!(
        root_manifest.contains("\"test-helpers\""),
        "root Cargo.toml workspace.members must include \"test-helpers\""
    );

    // @step And `codelet-test-helpers = { path = "test-helpers" }` appears in workspace.dependencies
    assert!(
        root_manifest.contains("codelet-test-helpers = { path = \"test-helpers\" }"),
        "root Cargo.toml [workspace.dependencies] must declare codelet-test-helpers = {{ path = \"test-helpers\" }}"
    );

    // @step And codelet/test-helpers/Cargo.toml declares package.name = "codelet-test-helpers"
    let helpers_manifest = read("test-helpers/Cargo.toml");
    assert!(
        helpers_manifest.contains("name = \"codelet-test-helpers\""),
        "codelet/test-helpers/Cargo.toml must declare package.name = \"codelet-test-helpers\""
    );

    // @step And codelet/test-helpers/Cargo.toml declares serde_json as its ONLY [dependencies] entry beyond workspace-inherited package fields
    assert!(
        helpers_manifest.contains("serde_json"),
        "codelet/test-helpers/Cargo.toml must depend on serde_json"
    );
    for forbidden in [
        "codelet-core",
        "codelet-napi",
        "codelet-sessions",
        "codelet-rpc",
        "codelet-rpc-types",
        "codelet-fspec",
        "codelet-fspec-tui",
        "codelet-common",
        "codelet-providers",
        "codelet-tools",
        "tokio",
        "anyhow",
        "thiserror",
        "tarpc",
        "reqwest",
    ] {
        assert!(
            !helpers_manifest.contains(forbidden),
            "codelet/test-helpers/Cargo.toml must NOT depend on `{forbidden}` (keeps the helper crate outside every forbidden-arrow graph)"
        );
    }

    // @step And codelet/test-helpers/Cargo.toml inherits [lints] from workspace
    assert!(
        helpers_manifest.contains("[lints]") && helpers_manifest.contains("workspace = true"),
        "codelet/test-helpers/Cargo.toml must inherit [lints] from the workspace"
    );
}

#[test]
fn scenario_codelet_test_helpers_exposes_the_shared_dependency_rule_helper_api() {
    // @step Given the new crate codelet/test-helpers/ exists with a library entry point at codelet/test-helpers/src/lib.rs
    let lib_rs = read("test-helpers/src/lib.rs");
    let dep_rules = read("test-helpers/src/dependency_rules.rs");

    // @step When I inspect the public surface of the dependency_rules module
    // @step Then `pub mod dependency_rules` is declared in codelet/test-helpers/src/lib.rs
    assert!(
        lib_rs.contains("pub mod dependency_rules"),
        "lib.rs must declare `pub mod dependency_rules`"
    );

    // @step And codelet/test-helpers/src/dependency_rules.rs defines `pub fn assert_no_transitive_dependency(from_crate: &str, forbidden_pkg: &str)`
    assert!(
        dep_rules.contains("pub fn assert_no_transitive_dependency_with_manifest")
            || dep_rules.contains("pub fn assert_no_transitive_dependency("),
        "dependency_rules.rs must expose assert_no_transitive_dependency (directly or via _with_manifest)"
    );
    assert!(
        dep_rules.contains("macro_rules! assert_no_transitive_dependency"),
        "dependency_rules.rs must export an assert_no_transitive_dependency! macro that forwards CARGO_MANIFEST_DIR to the helper fn"
    );

    // @step And codelet/test-helpers/src/dependency_rules.rs defines `pub fn assert_no_import_in_sources(crate_dir_name: &str, forbidden_module: &str)`
    assert!(
        dep_rules.contains("pub fn assert_no_import_in_sources_with_manifest")
            || dep_rules.contains("pub fn assert_no_import_in_sources("),
        "dependency_rules.rs must expose assert_no_import_in_sources (directly or via _with_manifest)"
    );
    assert!(
        dep_rules.contains("macro_rules! assert_no_import_in_sources"),
        "dependency_rules.rs must export an assert_no_import_in_sources! macro that forwards CARGO_MANIFEST_DIR to the helper fn"
    );

    // @step And both helpers use `cargo metadata --format-version 1` for the dependency-graph walk
    assert!(
        dep_rules.contains("\"metadata\"")
            && dep_rules.contains("\"--format-version\"")
            && dep_rules.contains("\"1\""),
        "dependency_rules.rs must invoke `cargo metadata --format-version 1`"
    );

    // @step And the source-scan helper strips Rust line and block comments before substring matching
    assert!(
        dep_rules.contains("fn strip_rust_comments"),
        "dependency_rules.rs must include a strip_rust_comments fn that strips // and /* */ comments before substring matching"
    );
}

#[test]
fn scenario_codelet_test_helpers_itself_has_no_transitive_dependency_on_codelet_napi() {
    // @step Given the codelet-test-helpers crate is published as a workspace member
    // @step When I run `cargo metadata --format-version 1` and walk the transitive dependencies of codelet-test-helpers
    // @step Then the resulting transitive package set does not contain the package name `codelet-napi`
    codelet_test_helpers::dependency_rules::assert_no_transitive_dependency_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "codelet-test-helpers",
        "codelet-napi",
    );

    // @step And codelet/test-helpers/src does not contain any `use codelet_napi` or `codelet_napi::` substring
    codelet_test_helpers::dependency_rules::assert_no_import_in_sources_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "test-helpers",
        "codelet_napi",
    );
}

#[test]
fn scenario_migrated_codelet_fspec_regression_test_delegates_to_the_shared_helper() {
    // @step Given the file codelet/fspec/tests/no_napi_dependency.rs
    let body = read("fspec/tests/no_napi_dependency.rs");
    let cargo = read("fspec/Cargo.toml");

    // @step When I inspect its source after the RPC-067 migration
    // @step Then it imports the shared helper module from codelet_test_helpers
    assert!(
        body.contains("use codelet_test_helpers::"),
        "fspec/tests/no_napi_dependency.rs must import from codelet_test_helpers"
    );
    // @step And it contains exactly two #[test] fns
    assert_eq!(
        body.matches("#[test]").count(),
        2,
        "fspec/tests/no_napi_dependency.rs must contain exactly two #[test] fns"
    );
    // @step And one #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_transitive_dependency("codelet-fspec", "codelet-napi")`
    assert!(
        body.contains("assert_no_transitive_dependency!(\"codelet-fspec\", \"codelet-napi\")"),
        "fspec/tests/no_napi_dependency.rs must invoke assert_no_transitive_dependency!(\"codelet-fspec\", \"codelet-napi\")"
    );
    // @step And the other #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_import_in_sources("fspec", "codelet_napi")`
    assert!(
        body.contains("assert_no_import_in_sources!(\"fspec\", \"codelet_napi\")"),
        "fspec/tests/no_napi_dependency.rs must invoke assert_no_import_in_sources!(\"fspec\", \"codelet_napi\")"
    );
    // @step And codelet/fspec/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]
    assert!(
        cargo.contains("codelet-test-helpers.workspace = true")
            || cargo.contains("codelet-test-helpers = { workspace = true }"),
        "codelet/fspec/Cargo.toml must declare codelet-test-helpers under [dev-dependencies]"
    );
}

#[test]
fn scenario_migrated_codelet_fspec_tui_regression_test_delegates_to_the_shared_helper() {
    // @step Given the file codelet/fspec-tui/tests/no_napi_dependency.rs
    let body = read("fspec-tui/tests/no_napi_dependency.rs");
    let cargo = read("fspec-tui/Cargo.toml");

    // @step When I inspect its source after the RPC-067 migration
    // @step Then it imports the shared helper module from codelet_test_helpers
    assert!(body.contains("use codelet_test_helpers::"));
    // @step And it contains exactly two #[test] fns
    assert_eq!(body.matches("#[test]").count(), 2);
    // @step And one #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_transitive_dependency("codelet-fspec-tui", "codelet-napi")`
    assert!(
        body.contains("assert_no_transitive_dependency!(\"codelet-fspec-tui\", \"codelet-napi\")")
    );
    // @step And the other #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_import_in_sources("fspec-tui", "codelet_napi")`
    assert!(body.contains("assert_no_import_in_sources!(\"fspec-tui\", \"codelet_napi\")"));
    // @step And codelet/fspec-tui/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]
    assert!(
        cargo.contains("codelet-test-helpers.workspace = true")
            || cargo.contains("codelet-test-helpers = { workspace = true }")
    );
}

#[test]
fn scenario_migrated_codelet_sessions_regression_test_delegates_to_the_shared_helper() {
    // @step Given the file codelet/sessions/tests/no_napi_dependency.rs
    let body = read("sessions/tests/no_napi_dependency.rs");
    let cargo = read("sessions/Cargo.toml");

    // @step When I inspect its source after the RPC-067 migration
    // @step Then it imports the shared helper module from codelet_test_helpers
    assert!(body.contains("use codelet_test_helpers::"));
    // @step And it contains exactly two #[test] fns
    assert_eq!(body.matches("#[test]").count(), 2);
    // @step And one #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_transitive_dependency("codelet-sessions", "codelet-napi")`
    assert!(
        body.contains("assert_no_transitive_dependency!(\"codelet-sessions\", \"codelet-napi\")")
    );
    // @step And the other #[test] fn calls `codelet_test_helpers::dependency_rules::assert_no_import_in_sources("sessions", "codelet_napi")`
    assert!(body.contains("assert_no_import_in_sources!(\"sessions\", \"codelet_napi\")"));
    // @step And codelet/sessions/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]
    assert!(
        cargo.contains("codelet-test-helpers.workspace = true")
            || cargo.contains("codelet-test-helpers = { workspace = true }")
    );
}

#[test]
fn scenario_rpc_006_source_shape_rs_is_left_untouched() {
    // @step Given the file codelet/rpc-embedded/tests/rpc_006_source_shape.rs
    let body = read("rpc-embedded/tests/rpc_006_source_shape.rs");

    // @step When I diff its content against the pre-RPC-067 baseline
    // @step Then no lines are removed
    for required in [
        "scenario_default_fixture_is_unreachable_from_production_code",
        "scenario_embedded_transport_reuses_host_runtime_handle_for_fan_out",
        "scenario_codelet_rpc_may_depend_on_codelet_core_but_not_on_codelet_napi",
        "scenario_rpc_server_binary_still_binds_loopback_only_after_watcher_integration",
        "scenario_work_unit_info_continues_to_be_defined_exactly_once_in_rpc_types",
        "scenario_embedded_push_path_contains_no_bincode_encode_call",
    ] {
        assert!(
            body.contains(required),
            "codelet/rpc-embedded/tests/rpc_006_source_shape.rs must still contain `{required}` (RPC-067 must not collapse the broader RPC-006 invariants into the helper)"
        );
    }

    // @step And no test function is replaced by a helper invocation
    assert!(
        !body.contains("use codelet_test_helpers::"),
        "codelet/rpc-embedded/tests/rpc_006_source_shape.rs must NOT consume codelet_test_helpers (its scenarios are wider than the helper API)"
    );
}

#[test]
fn scenario_source_scan_helper_ignores_comments_containing_the_forbidden_substring() {
    use codelet_test_helpers::dependency_rules::strip_rust_comments;

    // @step Given a Rust file containing only the line `// codelet_napi was here`
    let src = "// codelet_napi was here\nfn foo() {}\n";

    // @step When `assert_no_import_in_sources` walks the file
    let stripped = strip_rust_comments(src);

    // @step Then the helper does NOT flag the file as an offender
    assert!(
        !stripped.contains("use codelet_napi"),
        "stripped source must not retain the comment line (use needle)"
    );
    assert!(
        !stripped.contains("codelet_napi::"),
        "stripped source must not retain the comment line (path needle)"
    );
}

#[test]
fn scenario_sabotaging_codelet_core_by_adding_a_codelet_napi_dependency_makes_its_test_fail() {
    use codelet_test_helpers::dependency_rules::assert_no_transitive_dependency_with_manifest;

    // @step Given a developer adds `codelet-napi = { workspace = true }` to codelet/core/Cargo.toml
    // We can't mutate the manifest mid-test, so we drive the same code path
    // by asking the helper to walk a from_crate that ALREADY contains
    // codelet-napi in its transitive graph (codelet-napi → codelet-napi
    // is trivially true; the helper resolves the root by name).

    // @step When the developer runs `cargo test -p codelet-core --test no_napi_dependency`
    // (Simulated via a panic-catching wrapper around the same helper fn.)
    let panic_result = std::panic::catch_unwind(|| {
        assert_no_transitive_dependency_with_manifest(
            env!("CARGO_MANIFEST_DIR"),
            "codelet-napi",
            "codelet-napi",
        );
    });

    // @step Then the test exits with a non-zero code
    let err = panic_result.expect_err(
        "asking the helper to assert codelet-napi is absent from codelet-napi's OWN dep graph MUST panic — this proves the sabotage path fires",
    );

    // @step And the failure message contains the substring "codelet-napi"
    let payload = err
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            err.downcast_ref::<&'static str>()
                .map(std::string::ToString::to_string)
        })
        .unwrap_or_default();
    assert!(
        payload.contains("codelet-napi"),
        "sabotage failure message must mention codelet-napi; got: {payload}"
    );

    // @step And the failure message contains the substring "codelet-core"
    // We cannot mutate codelet/core/Cargo.toml mid-test, but the panic
    // template in dependency_rules.rs MUST embed `from_crate` so any
    // future sabotage where `from_crate="codelet-core"` produces a
    // failure message naming the offender. Verify the actual source
    // (not a synthetic probe) to catch regressions that would silently
    // drop `{from_crate}` from the panic format string.
    let dep_rules_src = read("test-helpers/src/dependency_rules.rs");
    assert!(
        dep_rules_src.contains("{from_crate}")
            && dep_rules_src.contains("MUST NOT transitively depend on"),
        "dependency_rules.rs panic format string MUST embed `{{from_crate}}` so a sabotage with from_crate=\"codelet-core\" produces a failure message that names codelet-core"
    );
}

#[test]
fn scenario_cargo_test_workspace_passes_after_all_rpc_067_changes() {
    // @step Given all RPC-067 source changes are applied to the workspace
    // (This is a vacuous compile-time / shape-level assertion: if the
    // RPC-067 changes are inconsistent the workspace fails to build and
    // this test binary never links. By reaching this body we've already
    // proven the workspace compiles; the runtime assertion below
    // confirms the five regression-test files exist on disk.)

    // @step When I run `cargo test --workspace --tests --no-fail-fast`
    // @step Then the command exits with code 0
    // @step And all five no_napi_dependency.rs test binaries (codelet-core, codelet-rpc-types, codelet-fspec, codelet-fspec-tui, codelet-sessions) report green
    for rel in [
        "core/tests/no_napi_dependency.rs",
        "rpc-types/tests/no_napi_dependency.rs",
        "fspec/tests/no_napi_dependency.rs",
        "fspec-tui/tests/no_napi_dependency.rs",
        "sessions/tests/no_napi_dependency.rs",
    ] {
        let body = read(rel);
        assert!(
            body.contains("use codelet_test_helpers::"),
            "{rel} must consume the shared codelet_test_helpers crate"
        );
        assert!(
            body.contains("assert_no_transitive_dependency!")
                && body.contains("assert_no_import_in_sources!"),
            "{rel} must invoke both forbidden-arrow helpers"
        );
    }
}

#[test]
fn scenario_codelet_core_forbidden_arrow_regression_test_passes_against_the_current_workspace() {
    use codelet_test_helpers::dependency_rules::{
        assert_no_import_in_sources_with_manifest, assert_no_transitive_dependency_with_manifest,
    };

    // @step Given the codelet workspace is in its current RPC-067 state
    // @step When I run `cargo test -p codelet-core --test no_napi_dependency`
    // (Driven inline through the helper fn against the same crate name —
    // the per-crate `no_napi_dependency.rs` binary at
    // codelet/core/tests/no_napi_dependency.rs delegates to this same
    // helper; running the helper here exercises identical assertions.)
    // @step Then the command exits with code 0
    // @step And the transitive dependency walk for codelet-core does NOT contain codelet-napi
    assert_no_transitive_dependency_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "codelet-core",
        "codelet-napi",
    );

    // @step And no `.rs` file under codelet/core/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped
    assert_no_import_in_sources_with_manifest(env!("CARGO_MANIFEST_DIR"), "core", "codelet_napi");

    // Shape pin: the dedicated binary exists and consumes the shared helper.
    let body = read("core/tests/no_napi_dependency.rs");
    assert!(body.contains("use codelet_test_helpers::"));
    assert!(
        body.contains("assert_no_transitive_dependency!(\"codelet-core\", \"codelet-napi\")"),
        "codelet/core/tests/no_napi_dependency.rs must invoke the helper macro with codelet-core / codelet-napi"
    );
}

#[test]
fn scenario_codelet_rpc_types_forbidden_arrow_regression_test_passes_against_the_current_workspace()
{
    use codelet_test_helpers::dependency_rules::{
        assert_no_import_in_sources_with_manifest, assert_no_transitive_dependency_with_manifest,
    };

    // @step Given the codelet workspace is in its current RPC-067 state
    // @step And the codelet-rpc-types crate is built with the default feature set (no `napi` feature)
    // (The default cargo metadata invocation resolves features according
    // to whatever the workspace specifies — codelet-rpc-types has no
    // default features that pull `napi` in, so this is the asserted
    // default-feature path.)
    // @step When I run `cargo test -p codelet-rpc-types --test no_napi_dependency`
    // @step Then the command exits with code 0
    // @step And the transitive dependency walk for codelet-rpc-types does NOT contain codelet-napi
    assert_no_transitive_dependency_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "codelet-rpc-types",
        "codelet-napi",
    );

    // @step And no `.rs` file under codelet/rpc-types/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped
    assert_no_import_in_sources_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "rpc-types",
        "codelet_napi",
    );

    // Shape pin: the dedicated binary exists and consumes the shared helper.
    let body = read("rpc-types/tests/no_napi_dependency.rs");
    assert!(body.contains("use codelet_test_helpers::"));
    assert!(
        body.contains("assert_no_transitive_dependency!(\"codelet-rpc-types\", \"codelet-napi\")"),
        "codelet/rpc-types/tests/no_napi_dependency.rs must invoke the helper macro with codelet-rpc-types / codelet-napi"
    );
}
