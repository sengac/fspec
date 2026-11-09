//! Feature: spec/features/rpc-types-field-level-cfg-attr-feature-napi-napi-js-name-uses-unscoped-napi-attribute.feature
//!
//! BUG-146 (Option B strategy): rpc-types previously decorated 34
//! struct/enum fields with
//!   `#[cfg_attr(feature = "napi", napi(js_name = "X"))]`
//! which fails to compile under the napi feature because Rust eagerly
//! validates the cfg_attr-expanded attribute path and rejects field-level
//! proc-macro attributes (see GH napi-rs#2635).
//!
//! Fix: replace each field-level decoration with
//!   `#[serde(rename = "X")]`.
//! napi-derive v3 is expected to honor `#[serde(rename)]` on
//! `#[napi(object)]` fields and produce the same camelCase TS surface.
//!
//! These tests cover the bug pre/post fix, the content shape of the
//! fixed on-disk source (BUG-150: formerly a time-bound git-diff pin),
//! the dependency cleanliness invariants, and the TS surface
//! preservation requirement.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// Path & helper functions.
// ---------------------------------------------------------------------------

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workspace_root() -> PathBuf {
    crate_root()
        .parent()
        .expect("rpc-types parent")
        .to_path_buf()
}

fn lib_rs_path() -> PathBuf {
    crate_root().join("src").join("lib.rs")
}

/// REFAC-021: lib.rs re-exports per-surface modules; read the whole tree.
fn read_lib_tree() -> String {
    codelet_test_helpers::source_composition::read_module_tree(&lib_rs_path())
        .unwrap_or_else(|e| panic!("failed to read rpc-types module tree: {e}"))
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Run a cargo command at the codelet workspace root.
fn run_cargo(args: &[&str]) -> std::process::Output {
    // BUG-317: build/test/check reuse the outer ci-test build instead of a
    // second dev-profile dependency tree (explicit --release is kept).
    let mut args: Vec<&str> = args.to_vec();
    if matches!(args.first(), Some(&("build" | "test" | "check"))) && !args.contains(&"--release") {
        args.splice(1..1, ["--profile", "ci-test"]);
    }
    Command::new("cargo")
        .args(&args)
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn cargo {args:?}: {e}"))
}

/// Camelcase names that were originally produced by `#[napi(js_name = "X")]`
/// at the 34 field sites. These must continue to appear in the regenerated
/// rust/napi/index.d.ts after the fix.
const EXPECTED_CAMEL_CASE_NAMES: &[&str] = &[
    "workType",
    "lastStateChangeAt",
    "timestampMs",
    "mediaType",
    "argsJson",
    "projectRoot",
    "toolCallId",
    "systemReminder",
    "correlationId",
    "observedCorrelationIds",
    "toolCall",
    "toolResult",
    "toolProgress",
    "queuedInputs",
    "contextFill",
    "supervisorPendingInjection",
    "compactionResult",
    "fspecRequest",
    "fspecResult",
    "workUnits",
    "isIsolated",
    "worktreePath",
    "baseCommit",
    "displayPath",
    "isGitRepo",
];

/// True if the fix has been applied — i.e. NO field-level
/// `cfg_attr(feature = "napi", napi(js_name = ...))` remains.
fn fix_is_applied(src: &str) -> bool {
    !src.contains("cfg_attr(feature = \"napi\", napi(js_name")
}

fn tail(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let start = s.len().saturating_sub(max);
        format!("...[truncated head]...\n{}", &s[start..])
    }
}

// ===========================================================================
// Scenario: The default codelet-rpc-types build remains free of napi-derive after the fix
// ===========================================================================

#[test]
fn scenario_post_fix_default_build_has_no_napi_dep() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read_lib_tree();
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `cargo build -p codelet-rpc-types`
    let out = run_cargo(&["build", "-p", "codelet-rpc-types"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // @step Then the build succeeds with exit code 0
    assert!(
        out.status.success(),
        "BUG-146: default `cargo build -p codelet-rpc-types` failed; stderr:\n{}",
        tail(&stderr, 4000),
    );

    // @step When I run `cargo tree -p codelet-rpc-types -e normal --no-default-features`
    let tree = run_cargo(&[
        "tree",
        "-p",
        "codelet-rpc-types",
        "-e",
        "normal",
        "--no-default-features",
    ]);
    let tree_stdout = String::from_utf8_lossy(&tree.stdout);
    assert!(tree.status.success(), "BUG-146: cargo tree failed");

    // @step Then the listed normal dependencies do NOT include `napi` or `napi-derive`
    for line in tree_stdout.lines() {
        let trimmed = line.trim_start_matches(|c: char| !c.is_alphabetic());
        if trimmed.starts_with("napi ") || trimmed.starts_with("napi-derive ") {
            panic!(
                "BUG-146: cargo tree --no-default-features must NOT list `napi` or `napi-derive`; offending line: {line}\nfull output:\n{tree_stdout}",
            );
        }
    }
}

// ===========================================================================
// Scenario: The fix replaces every field-level napi(js_name) with serde(rename) in the on-disk source
// ===========================================================================

/// BUG-150: this scenario formerly pinned the shape of the UNCOMMITTED git
/// working-tree diff of lib.rs (exactly 34 added `serde(rename` lines and
/// 33-34 removed `cfg_attr...napi(js_name` lines). That was a time-bound
/// git-state assertion: it only held in the specific tree where the BUG-146
/// fix sat uncommitted, and went red on any diverging tree (after the fix
/// was committed, or when CONT-007/CONT-008 added new legitimate
/// `serde(rename` lines). It is now a content-based pin on the on-disk
/// source: git is never consulted, so the outcome is independent of git
/// state while still failing if any documented BUG-146 rename regresses.
#[test]
fn scenario_fix_replaces_field_level_attrs_and_no_other_changes() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read_lib_tree();
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I inspect the on-disk contents of rust/rpc-types/src/lib.rs
    let code_lines: Vec<&str> = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect();

    // @step Then the source contains zero field-level `napi(js_name` attributes outside comments
    let offending: Vec<&str> = code_lines
        .iter()
        .filter(|l| l.contains("napi(js_name"))
        .copied()
        .collect();
    assert!(
        offending.is_empty(),
        "BUG-146/BUG-150: lib.rs must contain ZERO non-comment `napi(js_name` attributes; offending lines:\n{}",
        offending.join("\n"),
    );

    // @step And each of the 34 documented renames X appears as `#[serde(rename = "X")]` with at least its documented multiplicity
    //
    // The 34 documented field sites map onto 25 distinct names; three names
    // are renamed at multiple sites (correlationId x5,
    // observedCorrelationIds x5, toolCallId x2; 22 names x1 → 34 sites).
    // Later legitimate work may ADD renames, so the pin is "at least the
    // documented multiplicity" — it never goes stale on correct trees but
    // still fails if a documented rename is removed or reverted.
    let documented_multiplicity = |name: &str| -> usize {
        match name {
            "correlationId" | "observedCorrelationIds" => 5,
            "toolCallId" => 2,
            _ => 1,
        }
    };
    // BUG-150 O1: count over `code_lines` (comment lines filtered), matching
    // the zero-`napi(js_name` check above — a `//` comment mentioning a
    // rename must neither satisfy nor inflate the pin.
    let mut violations: Vec<String> = Vec::new();
    for name in EXPECTED_CAMEL_CASE_NAMES {
        let needle = format!("#[serde(rename = \"{name}\"");
        let count: usize = code_lines
            .iter()
            .map(|l| l.matches(needle.as_str()).count())
            .sum();
        let want = documented_multiplicity(name);
        if count < want {
            violations.push(format!(
                "rename \"{name}\": expected at least {want} `{needle}...` occurrence(s) in lib.rs, found {count}"
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "BUG-146/BUG-150: documented serde rename(s) regressed:\n{}",
        violations.join("\n"),
    );

    // @step And the source contains at least 34 `#[serde(rename = ` attributes in total
    // BUG-150 O1: same comment-line filtering as the per-name counts above.
    let serde_rename_count: usize = code_lines
        .iter()
        .map(|l| l.matches("#[serde(rename = ").count())
        .sum();
    assert!(
        serde_rename_count >= 34,
        "BUG-146: lib.rs must contain at least 34 `#[serde(rename = ` lines (got {serde_rename_count})",
    );

    // @step And NO `use napi_derive::napi;` import is present in the source
    assert!(
        !code_lines
            .iter()
            .any(|l| l.contains("use napi_derive::napi")),
        "BUG-146: lib.rs must NOT contain `use napi_derive::napi;`",
    );

    // @step And the struct-level `napi_derive::napi(object)` decorations remain in place
    let object_decorations = code_lines
        .iter()
        .filter(|l| l.contains("#[derive(") && l.contains("Serialize, Deserialize"))
        .count();
    assert!(
        object_decorations >= 40,
        "BUG-146: struct-level serde derives must remain on the wire types (documented ~40 sites; got {object_decorations})",
    );

    // @step When I inspect the on-disk contents of rust/rpc-types/Cargo.toml
    let toml = read(&crate_root().join("Cargo.toml"));

    // @step Then the napi and napi-derive dependencies remain optional and gated behind the napi feature
    let napi_dep_lines: Vec<&str> = toml
        .lines()
        .filter(|l| {
            let key = l.trim_start();
            key.starts_with("napi = ") || key.starts_with("napi-derive = ")
        })
        .collect();
    assert!(
        napi_dep_lines.is_empty(),
        "BUG-146/REFAC-027: rpc-types Cargo.toml must not declare napi or napi-derive dependencies; found: {napi_dep_lines:?}",
    );
}
