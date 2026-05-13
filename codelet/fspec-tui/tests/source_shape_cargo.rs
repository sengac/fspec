//! Source-shape regression tests for codelet/fspec-tui Cargo.toml + workspace
//! registration + runtime-construction invariants (RPC-008).
//!
//! Feature: spec/features/fspec-tui-cargo-shape.feature
//!
//! Scenarios covered (Cargo / workspace half):
//!   - "codelet/fspec-tui is a fifth RPC-family workspace member with no
//!     binary entry point"
//!   - "codelet/fspec-tui production dependencies include only RPC seam
//!     crates plus ratatui dependencies"
//!   - "codelet/fspec-tui dev-dependencies allow codelet-core for fixtures
//!     but never codelet-napi"
//!   - "codelet/fspec-tui contains no own-runtime construction calls"
//!   - "The existing scenario_7_* test in rpc-embedded is widened to scan
//!     fspec-tui"
//!
//! These tests do not exercise any runtime code path. The trait-shape half
//! of the source-shape regressions lives in
//! `tests/source_shape_trait.rs` to keep each file under the project's
//! 300-LoC ceiling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

#[test]
fn codelet_fspec_tui_is_a_workspace_member_with_no_binary_entry_point() {
    // @step Given the codelet workspace currently lists rpc, rpc-types, rpc-embedded, and rpc-server as RPC-family members
    let workspace_cargo_toml =
        common::read_to_string_or_panic(&common::workspace_root().join("Cargo.toml"));

    // @step When I add codelet/fspec-tui to codelet/Cargo.toml's [workspace] members and create codelet/fspec-tui/Cargo.toml plus codelet/fspec-tui/src/lib.rs
    // (already done by the RPC-008 commit; this assertion is the post-condition)

    // @step Then "codelet-fspec-tui" appears in the workspace member list
    assert!(
        workspace_cargo_toml.contains("\"fspec-tui\""),
        "codelet/Cargo.toml [workspace] members must include \"fspec-tui\". Got:\n{workspace_cargo_toml}"
    );

    // @step And codelet/fspec-tui/Cargo.toml contains a [lib] section but NO [[bin]] section
    let fspec_tui_cargo_toml = common::read_to_string_or_panic(
        &common::workspace_root().join("fspec-tui").join("Cargo.toml"),
    );
    assert!(
        fspec_tui_cargo_toml.contains("[lib]"),
        "fspec-tui Cargo.toml must declare [lib]"
    );
    assert!(
        !fspec_tui_cargo_toml.contains("[[bin]]"),
        "fspec-tui Cargo.toml MUST NOT declare any [[bin]] section in this card"
    );

    // @step And running "cargo build -p codelet-fspec-tui" from codelet/ exits with status code 0
    // Implicit: this integration test is only running because the test
    // binary was compiled, which requires `cargo build -p codelet-fspec-tui`
    // to have succeeded.
    let lib_rs = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("lib.rs");
    assert!(
        lib_rs.exists(),
        "expected codelet/fspec-tui/src/lib.rs to exist"
    );
}

#[test]
fn codelet_fspec_tui_production_dependencies_include_only_rpc_seam_and_ratatui_deps() {
    // @step Given codelet/fspec-tui/Cargo.toml exists
    let cargo_toml_path = common::workspace_root().join("fspec-tui").join("Cargo.toml");
    let raw = common::read_to_string_or_panic(&cargo_toml_path);

    // @step When I read its [dependencies] table
    let deps_section = section_body(&raw, "[dependencies]");

    // @step Then the [dependencies] table contains exactly these workspace dependencies: codelet-rpc, codelet-rpc-types, codelet-rpc-embedded, codelet-rpc-server, ratatui, crossterm, tokio, async-trait, futures, tarpc, tokio-tungstenite, url, anyhow, tracing
    for required in [
        "codelet-rpc",
        "codelet-rpc-types",
        "codelet-rpc-embedded",
        "codelet-rpc-server",
        "ratatui",
        "crossterm",
        "tokio",
        "async-trait",
        "futures",
        "tarpc",
        "tokio-tungstenite",
        "url",
        "anyhow",
        "tracing",
    ] {
        assert!(
            deps_section.contains(required),
            "[dependencies] table must list `{required}`. Section:\n{deps_section}"
        );
    }

    // @step And the [dependencies] table contains "tui-popup" pinned to "0.6"
    assert!(
        deps_section.contains("tui-popup"),
        "[dependencies] must list tui-popup"
    );
    assert!(
        deps_section.contains("\"0.6\"") || deps_section.contains("= \"0.6\""),
        "[dependencies] tui-popup must be pinned to \"0.6\""
    );

    // @step And the [dependencies] table does NOT list codelet-napi
    assert!(
        !contains_dep_key(deps_section, "codelet-napi"),
        "[dependencies] MUST NOT list codelet-napi"
    );

    // @step And the [dependencies] table does NOT list codelet-core
    assert!(
        !contains_dep_key(deps_section, "codelet-core"),
        "[dependencies] MUST NOT list codelet-core (production code uses the RPC seam only)"
    );
}

#[test]
fn codelet_fspec_tui_dev_dependencies_allow_codelet_core_for_fixtures_but_never_napi() {
    // @step Given codelet/fspec-tui/Cargo.toml exists
    let cargo_toml_path = common::workspace_root().join("fspec-tui").join("Cargo.toml");
    let raw = common::read_to_string_or_panic(&cargo_toml_path);

    // @step When I read its [dev-dependencies] table
    let dev_section = section_body(&raw, "[dev-dependencies]");

    // @step Then the [dev-dependencies] table contains insta with the yaml feature
    assert!(
        dev_section.contains("insta"),
        "[dev-dependencies] must list insta. Section:\n{dev_section}"
    );

    // @step And the [dev-dependencies] table contains tempfile and tokio-test
    assert!(
        dev_section.contains("tempfile"),
        "[dev-dependencies] must list tempfile"
    );
    assert!(
        dev_section.contains("tokio-test"),
        "[dev-dependencies] must list tokio-test"
    );

    // @step And the [dev-dependencies] table MAY list codelet-core (for real-service integration fixtures)
    assert!(
        contains_dep_key(dev_section, "codelet-core"),
        "[dev-dependencies] MUST list codelet-core to support the temp_service fixture (real WorkUnitsWatcher)"
    );

    // @step And the [dev-dependencies] table does NOT list codelet-napi
    assert!(
        !contains_dep_key(dev_section, "codelet-napi"),
        "[dev-dependencies] MUST NOT list codelet-napi"
    );
}

#[test]
fn codelet_fspec_tui_contains_no_own_runtime_construction_calls() {
    // @step Given a source-shape integration test that scans codelet/fspec-tui/src/
    let src_dir = common::workspace_root().join("fspec-tui").join("src");
    let rs_files = common::collect_rs_files(&src_dir);
    assert!(
        !rs_files.is_empty(),
        "expected at least one .rs file under codelet/fspec-tui/src/"
    );

    // @step When the test reads each .rs file and strips comments
    let mut violations: Vec<String> = Vec::new();
    for path in &rs_files {
        let body = common::read_to_string_or_panic(path);
        let code = common::strip_rust_comments(&body);
        for needle in [
            "tokio::runtime::Builder",
            "runtime::Builder::new_multi_thread",
            "runtime::Builder::new_current_thread",
            "tokio::runtime::Runtime::new",
            "Runtime::new()",
        ] {
            if code.contains(needle) {
                violations.push(format!("{}: {}", path.display(), needle));
            }
        }
    }

    // @step Then no file contains "tokio::runtime::Builder"
    // @step And no file contains "runtime::Builder::new_multi_thread"
    // @step And no file contains "runtime::Builder::new_current_thread"
    // @step And no file contains "tokio::runtime::Runtime::new"
    // @step And no file contains "Runtime::new()"
    assert!(
        violations.is_empty(),
        "codelet/fspec-tui MUST NOT construct its own tokio runtime. \
         The host-supplied tokio::runtime::Handle invariant from RPC-005 Q9 \
         applies to this layer too. Violations: {violations:?}"
    );
}

#[test]
fn rpc_embedded_scenario_7_is_widened_to_scan_fspec_tui() {
    // @step Given the existing test codelet/rpc-embedded/tests/architecture_invariants.rs::scenario_7_embedded_transport_requires_tokio_handle_at_construction
    let test_path = common::workspace_root()
        .join("rpc-embedded")
        .join("tests")
        .join("architecture_invariants.rs");
    let body = common::read_to_string_or_panic(&test_path);

    // @step When I inspect its directory list
    // @step Then it scans both "rpc-embedded/src" AND "fspec-tui/src" for forbidden runtime construction calls
    assert!(
        body.contains("\"fspec-tui\"")
            || body.contains("\"fspec-tui/src\"")
            || body.contains("fspec-tui"),
        "scenario_7_* in rpc-embedded must be widened to scan fspec-tui. \
         Searched body of {} and did not find any reference to fspec-tui",
        test_path.display()
    );
    assert!(
        body.contains("rpc-embedded"),
        "the original rpc-embedded scan target must remain after widening"
    );

    // @step And the assertion message identifies which crate violated the invariant on failure
    let mentions_both_paths = body.contains("rpc-embedded") && body.contains("fspec-tui");
    assert!(
        mentions_both_paths,
        "widened scenario_7_* must mention both rpc-embedded and fspec-tui in its body \
         so failure messages identify which crate violated the invariant"
    );
}

/// Sub-section extractor: returns the substring of `raw` from after
/// `header` up to (but not including) the next `[section]` header. Used
/// by the Cargo.toml dependency-shape tests above.
fn section_body<'a>(raw: &'a str, header: &str) -> &'a str {
    let Some(start) = raw.find(header) else {
        panic!(
            "expected section header `{header}` to appear in Cargo.toml. Got:\n{raw}"
        );
    };
    let after_header = &raw[start + header.len()..];
    if let Some(next) = after_header.find("\n[") {
        &after_header[..next]
    } else {
        after_header
    }
}

/// Returns true iff `section` contains a key line matching `name`,
/// excluding comments and string-literal false positives. Used by the
/// dependency-shape tests above.
fn contains_dep_key(section: &str, name: &str) -> bool {
    section.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            return false;
        }
        let prefix = trimmed.split('=').next().unwrap_or("").trim();
        let prefix_no_dot = prefix.split('.').next().unwrap_or("").trim();
        prefix_no_dot == name
    })
}
