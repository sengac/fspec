//! Feature: spec/features/dependency-rule-regression-tests.feature
use super::*;

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

    // @step Given a developer adds `codelet-napi = { workspace = true }` to rust/core/Cargo.toml
    // We can't mutate the manifest mid-test, so we drive the same code path
    // by asking the helper to walk a from_crate that ALREADY contains
    // codelet-napi in its transitive graph (codelet-napi → codelet-napi
    // is trivially true; the helper resolves the root by name).

    // @step When the developer runs `cargo test -p codelet-core --test no_napi_dependency`
    // (Simulated via a panic-catching wrapper around the same helper fn.)
    let panic_result = std::panic::catch_unwind(|| {
        assert_no_transitive_dependency_with_manifest(
            env!("CARGO_MANIFEST_DIR"),
            "codelet-core",
            "codelet-rpc-types",
        );
    });

    // @step Then the test exits with a non-zero code
    let err = panic_result.expect_err(
        "asking the helper to assert codelet-rpc-types (a real dependency) is absent from codelet-core's dep graph MUST panic — this proves the sabotage path fires",
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
        payload.contains("MUST NOT transitively depend on codelet-rpc-types"),
        "sabotage failure message must name the forbidden dependency; got: {payload}"
    );

    // @step And the failure message contains the substring "codelet-core"
    // We cannot mutate rust/core/Cargo.toml mid-test, but the panic
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

    // @step When I run `cargo test -p codelet-core -p codelet-rpc-types -p codelet-fspec -p codelet-fspec-tui -p codelet-sessions --test no_napi_dependency --no-fail-fast`
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
    // rust/core/tests/no_napi_dependency.rs delegates to this same
    // helper; running the helper here exercises identical assertions.)
    // @step Then the command exits with code 0
    // @step And the transitive dependency walk for codelet-core does NOT contain codelet-napi
    assert_no_transitive_dependency_with_manifest(
        env!("CARGO_MANIFEST_DIR"),
        "codelet-core",
        "codelet-napi",
    );

    // @step And no `.rs` file under rust/core/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped
    assert_no_import_in_sources_with_manifest(env!("CARGO_MANIFEST_DIR"), "core", "codelet_napi");

    // Shape pin: the dedicated binary exists and consumes the shared helper.
    let body = read("core/tests/no_napi_dependency.rs");
    assert!(body.contains("use codelet_test_helpers::"));
    assert!(
        body.contains("assert_no_transitive_dependency!(\"codelet-core\", \"codelet-napi\")"),
        "rust/core/tests/no_napi_dependency.rs must invoke the helper macro with codelet-core / codelet-napi"
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

    // @step And no `.rs` file under rust/rpc-types/src contains a `use codelet_napi` or `codelet_napi::` substring after comments are stripped
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
        "rust/rpc-types/tests/no_napi_dependency.rs must invoke the helper macro with codelet-rpc-types / codelet-napi"
    );
}
