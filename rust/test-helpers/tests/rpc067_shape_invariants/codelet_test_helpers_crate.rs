//! Feature: spec/features/dependency-rule-regression-tests.feature
use super::*;

#[test]
fn scenario_codelet_test_helpers_crate_is_wired_into_the_workspace_as_a_library() {
    // @step Given the codelet workspace root manifest at rust/Cargo.toml
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

    // @step And rust/test-helpers/Cargo.toml declares package.name = "codelet-test-helpers"
    let helpers_manifest = read("test-helpers/Cargo.toml");
    assert!(
        helpers_manifest.contains("name = \"codelet-test-helpers\""),
        "rust/test-helpers/Cargo.toml must declare package.name = \"codelet-test-helpers\""
    );

    // @step And rust/test-helpers/Cargo.toml declares serde_json as its ONLY [dependencies] entry beyond workspace-inherited package fields
    assert!(
        helpers_manifest.contains("serde_json"),
        "rust/test-helpers/Cargo.toml must depend on serde_json"
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
            "rust/test-helpers/Cargo.toml must NOT depend on `{forbidden}` (keeps the helper crate outside every forbidden-arrow graph)"
        );
    }

    // @step And rust/test-helpers/Cargo.toml inherits [lints] from workspace
    assert!(
        helpers_manifest.contains("[lints]") && helpers_manifest.contains("workspace = true"),
        "rust/test-helpers/Cargo.toml must inherit [lints] from the workspace"
    );
}

#[test]
fn scenario_codelet_test_helpers_exposes_the_shared_dependency_rule_helper_api() {
    // @step Given the new crate rust/test-helpers/ exists with a library entry point at rust/test-helpers/src/lib.rs
    let lib_rs = read("test-helpers/src/lib.rs");
    let dep_rules = read("test-helpers/src/dependency_rules.rs");

    // @step When I inspect the public surface of the dependency_rules module
    // @step Then `pub mod dependency_rules` is declared in rust/test-helpers/src/lib.rs
    assert!(
        lib_rs.contains("pub mod dependency_rules"),
        "lib.rs must declare `pub mod dependency_rules`"
    );

    // @step And rust/test-helpers/src/dependency_rules.rs defines `pub fn assert_no_transitive_dependency(from_crate: &str, forbidden_pkg: &str)`
    assert!(
        dep_rules.contains("pub fn assert_no_transitive_dependency_with_manifest")
            || dep_rules.contains("pub fn assert_no_transitive_dependency("),
        "dependency_rules.rs must expose assert_no_transitive_dependency (directly or via _with_manifest)"
    );
    assert!(
        dep_rules.contains("macro_rules! assert_no_transitive_dependency"),
        "dependency_rules.rs must export an assert_no_transitive_dependency! macro that forwards CARGO_MANIFEST_DIR to the helper fn"
    );

    // @step And rust/test-helpers/src/dependency_rules.rs defines `pub fn assert_no_import_in_sources(crate_dir_name: &str, forbidden_module: &str)`
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

    // @step And rust/test-helpers/src does not contain any `use codelet_napi` or `codelet_napi::` substring
    codelet_test_helpers::dependency_rules::assert_no_import_in_sources_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "test-helpers",
        "codelet_napi",
    );
}

#[test]
fn scenario_migrated_codelet_fspec_regression_test_delegates_to_the_shared_helper() {
    // @step Given the file rust/fspec/tests/no_napi_dependency.rs
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
    // @step And rust/fspec/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]
    assert!(
        cargo.contains("codelet-test-helpers.workspace = true")
            || cargo.contains("codelet-test-helpers = { workspace = true }"),
        "rust/fspec/Cargo.toml must declare codelet-test-helpers under [dev-dependencies]"
    );
}

#[test]
fn scenario_migrated_codelet_fspec_tui_regression_test_delegates_to_the_shared_helper() {
    // @step Given the file rust/fspec-tui/tests/no_napi_dependency.rs
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
    // @step And rust/fspec-tui/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]
    assert!(
        cargo.contains("codelet-test-helpers.workspace = true")
            || cargo.contains("codelet-test-helpers = { workspace = true }")
    );
}

#[test]
fn scenario_migrated_codelet_sessions_regression_test_delegates_to_the_shared_helper() {
    // @step Given the file rust/sessions/tests/no_napi_dependency.rs
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
    // @step And rust/sessions/Cargo.toml declares `codelet-test-helpers.workspace = true` under [dev-dependencies]
    assert!(
        cargo.contains("codelet-test-helpers.workspace = true")
            || cargo.contains("codelet-test-helpers = { workspace = true }")
    );
}

#[test]
fn scenario_rpc_006_source_shape_rs_is_left_untouched() {
    // @step Given the file rust/rpc-embedded/tests/rpc_006_source_shape.rs
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
            "rust/rpc-embedded/tests/rpc_006_source_shape.rs must still contain `{required}` (RPC-067 must not collapse the broader RPC-006 invariants into the helper)"
        );
    }

    // @step And no test function is replaced by a helper invocation
    assert!(
        !body.contains("use codelet_test_helpers::"),
        "rust/rpc-embedded/tests/rpc_006_source_shape.rs must NOT consume codelet_test_helpers (its scenarios are wider than the helper API)"
    );
}
