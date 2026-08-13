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

fn napi_index_dts_path() -> PathBuf {
    workspace_root().join("napi").join("index.d.ts")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// Run a cargo command at the codelet workspace root.
fn run_cargo(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .args(args)
        .current_dir(workspace_root())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn cargo {args:?}: {e}"))
}

/// The 34 field-level cfg_attr line numbers documented in the bug.
const FIELD_LEVEL_LINES: &[u32] = &[
    40, 58, 217, 426, 457, 459, 461, 471, 473, 705, 707, 712, 714, 718, 720, 722, 726, 728, 730,
    734, 736, 738, 749, 756, 771, 775, 779, 783, 787, 791, 793, 799, 804, 806,
];

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
// Scenario: Reproducing the bug before the fix shows 34 unscoped-napi-attribute errors
// ===========================================================================

#[test]
fn scenario_repro_before_fix_shows_34_unscoped_napi_errors() {
    // @step Given I am at the codelet workspace root
    let _root = workspace_root();

    // @step And the file rust/rpc-types/src/lib.rs still contains 34 `#[cfg_attr(feature = "napi", napi(js_name = "..."))]` field-level decorations
    let src = read(&lib_rs_path());
    if fix_is_applied(&src) {
        // Post-fix path: precondition no longer holds → scenario obsolete,
        // pass trivially. The post-fix scenarios assert the positive case.
        return;
    }

    // @step When I run `cargo build -p codelet-napi --features noop`
    let out = run_cargo(&["build", "-p", "codelet-napi", "--features", "noop"]);

    // @step Then the build fails
    assert!(
        !out.status.success(),
        "BUG-146 (pre-fix): expected `cargo build -p codelet-napi --features noop` to FAIL but it succeeded",
    );

    let stderr = String::from_utf8_lossy(&out.stderr);

    // @step And the stderr contains 34 errors of the form "cannot find attribute `napi` in this scope"
    let needle = "cannot find attribute `napi` in this scope";
    let count = stderr.matches(needle).count();
    assert_eq!(
        count,
        34,
        "BUG-146 (pre-fix): expected EXACTLY 34 `{needle}` errors, found {count}; stderr tail:\n{}",
        tail(&stderr, 6000),
    );

    // @step And each error points at one of the 34 known field-level cfg_attr sites in lib.rs
    for line in FIELD_LEVEL_LINES {
        let marker = format!("lib.rs:{line}");
        assert!(
            stderr.contains(&marker),
            "BUG-146 (pre-fix): expected stderr to point at lib.rs:{line}; stderr tail:\n{}",
            tail(&stderr, 6000),
        );
    }
}

// ===========================================================================
// Scenario: After the fix, codelet-rpc-types builds cleanly with the napi feature enabled
// ===========================================================================

#[test]
fn scenario_post_fix_napi_feature_build_succeeds() {
    // @step Given the fix (replacing every field-level `napi(js_name = "X")` with `serde(rename = "X")`) has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(
        fix_is_applied(&src),
        "BUG-146: lib.rs still contains `cfg_attr(feature = \"napi\", napi(js_name` — fix has not been applied yet",
    );

    // @step When I run `cargo build -p codelet-rpc-types --features napi`
    let out = run_cargo(&["build", "-p", "codelet-rpc-types", "--features", "napi"]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // @step Then the build succeeds with exit code 0
    assert!(
        out.status.success(),
        "BUG-146: `cargo build -p codelet-rpc-types --features napi` failed; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the stderr contains "Compiling codelet-rpc-types" or "Finished"
    assert!(
        stderr.contains("Compiling codelet-rpc-types") || stderr.contains("Finished"),
        "BUG-146: stderr must mention `Compiling codelet-rpc-types` or `Finished`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the stderr ends with a "Finished" line
    assert!(
        stderr.contains("Finished"),
        "BUG-146: stderr must contain `Finished`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the stderr does NOT contain "cannot find attribute `napi`"
    assert!(
        !stderr.contains("cannot find attribute `napi`"),
        "BUG-146: stderr must NOT contain `cannot find attribute napi`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the stderr does NOT contain "expected non-macro attribute"
    assert!(
        !stderr.contains("expected non-macro attribute"),
        "BUG-146: stderr must NOT contain `expected non-macro attribute`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );
}

// ===========================================================================
// Scenario: The default codelet-rpc-types build remains free of napi-derive after the fix
// ===========================================================================

#[test]
fn scenario_post_fix_default_build_has_no_napi_dep() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
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
// Scenario: codelet-napi default features build succeeds after the fix
// ===========================================================================

#[test]
fn scenario_post_fix_codelet_napi_default_build_succeeds() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `cargo build -p codelet-napi`
    let out = run_cargo(&["build", "-p", "codelet-napi"]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // @step Then the build succeeds with exit code 0
    assert!(
        out.status.success(),
        "BUG-146: `cargo build -p codelet-napi` failed; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the stderr no longer contains "cannot find attribute `napi`"
    assert!(
        !stderr.contains("cannot find attribute `napi`"),
        "BUG-146: stderr must NOT contain `cannot find attribute napi`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );
}

// ===========================================================================
// Scenario: codelet-napi noop feature build succeeds after the fix
// ===========================================================================

#[test]
fn scenario_post_fix_codelet_napi_noop_build_succeeds() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `cargo build -p codelet-napi --features noop`
    let out = run_cargo(&["build", "-p", "codelet-napi", "--features", "noop"]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // @step Then the build succeeds with exit code 0
    assert!(
        out.status.success(),
        "BUG-146: `cargo build -p codelet-napi --features noop` failed; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the stderr no longer contains "cannot find attribute `napi`"
    assert!(
        !stderr.contains("cannot find attribute `napi`"),
        "BUG-146: stderr must NOT contain `cannot find attribute napi`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );
}

// ===========================================================================
// Scenario: codelet-rpc-types JSON round-trip tests still pass with the napi feature
// ===========================================================================

#[test]
fn scenario_post_fix_rpc036_tests_pass_with_napi_feature() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I type-check the test suite with the napi feature enabled via `cargo check -p codelet-rpc-types --features napi --tests`
    //
    // RATIONALE (BUG-150 W2): `cargo test --features napi` would require
    // LINKING the test binary against the napi/Node.js runtime, which is
    // not present at host-side test execution time (those tests would only
    // run under node-with-napi-loaded). The Gherkin therefore specifies a
    // type-check of the test suite: the type system + serde + napi_derive
    // macros must all expand and type-check cleanly.
    let out = run_cargo(&[
        "check",
        "-p",
        "codelet-rpc-types",
        "--features",
        "napi",
        "--tests",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // @step Then the type check succeeds with exit code 0
    assert!(
        out.status.success(),
        "BUG-146: `cargo check -p codelet-rpc-types --features napi --tests` failed; stderr tail:\n{}\nstdout tail:\n{}",
        tail(&stderr, 4000),
        tail(&stdout, 4000),
    );

    // @step And the output contains no compile errors
    assert!(
        !stderr.contains("error[E") && !stderr.contains("error: "),
        "BUG-146: stderr must NOT contain compile errors; stderr tail:\n{}",
        tail(&stderr, 4000),
    );
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
    let src = read(&lib_rs_path());
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
    let object_decorations = src.matches("napi_derive::napi(object)").count();
    assert!(
        object_decorations >= 40,
        "BUG-146: struct-level `napi_derive::napi(object)` decorations must remain (documented ~40 sites; got {object_decorations})",
    );

    // @step When I inspect the on-disk contents of rust/rpc-types/Cargo.toml
    let toml = read(&crate_root().join("Cargo.toml"));

    // @step Then the napi and napi-derive dependencies remain optional and gated behind the napi feature
    assert!(
        toml.contains("napi = [\"dep:napi\", \"dep:napi-derive\"]"),
        "BUG-146: Cargo.toml must keep the `napi` feature gating `dep:napi` and `dep:napi-derive`",
    );
    let napi_dep_optional = toml
        .lines()
        .any(|l| l.trim_start().starts_with("napi = {") && l.contains("optional = true"));
    let napi_derive_optional = toml
        .lines()
        .any(|l| l.trim_start().starts_with("napi-derive = {") && l.contains("optional = true"));
    assert!(
        napi_dep_optional && napi_derive_optional,
        "BUG-146: napi and napi-derive must remain optional dependencies in Cargo.toml",
    );
}

// ===========================================================================
// Scenario: TypeScript surface preserves every camelCase field after regeneration
// ===========================================================================

#[test]
fn scenario_index_dts_preserves_camelcase_field_names() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `cargo build -p codelet-napi --release` to regenerate rust/napi/index.d.ts
    let out = run_cargo(&["build", "-p", "codelet-napi", "--release"]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "BUG-146: `cargo build -p codelet-napi --release` failed; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step Then the regenerated index.d.ts contains every expected camelCase field name from the 34 original renames
    let dts_path = napi_index_dts_path();
    assert!(
        dts_path.exists(),
        "BUG-146: rust/napi/index.d.ts must exist at {}; release build must regenerate it",
        dts_path.display(),
    );
    let dts = read(&dts_path);

    let mut missing: Vec<&'static str> = Vec::new();
    for name in EXPECTED_CAMEL_CASE_NAMES {
        // Look for `<name>:` or `<name>?:` (optional field) — robust against
        // both required and optional TS field declarations. Also allow
        // `readonly ` prefix.
        let needles = [
            format!("{name}:"),
            format!("{name}?:"),
            format!("readonly {name}:"),
            format!("readonly {name}?:"),
        ];
        if !needles.iter().any(|n| dts.contains(n)) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "BUG-146: regenerated index.d.ts is missing {} expected camelCase field names: {:?}. This indicates napi-derive v3 did NOT honor `#[serde(rename)]` for napi(object) fields — the fix must fall back to Option (c) (struct duplication).",
        missing.len(),
        missing,
    );

    // @step And the camelCase names appear in the same struct positions they did before
    // (Stronger structural assertion is left to the RPC-043 byte-stability
    // assertion. Here we settle for: every name appears at least once and
    // none of the original snake_case names appears in their place at the
    // TS surface.)
    let forbidden_snake_case = [
        "work_type",
        "last_state_change_at",
        "timestamp_ms",
        "media_type",
        "args_json",
        "project_root",
        "tool_call_id",
        "system_reminder",
        "correlation_id",
        "observed_correlation_ids",
        "tool_call",
        "tool_result",
        "tool_progress",
        "queued_inputs",
        "context_fill",
        "supervisor_pending_injection",
        "compaction_result",
        "fspec_request",
        "fspec_result",
        "work_units",
        "is_isolated",
        "worktree_path",
        "base_commit",
        "display_path",
        "is_git_repo",
    ];
    let mut leaked: Vec<&'static str> = Vec::new();
    for snake in &forbidden_snake_case {
        let needle = format!("{snake}:");
        if dts.contains(&needle) {
            leaked.push(snake);
        }
    }
    assert!(
        leaked.is_empty(),
        "BUG-146: regenerated index.d.ts leaks snake_case field names {leaked:?} that should have been renamed to camelCase. This means the serde rename was not picked up by napi-derive.",
    );
}

// ===========================================================================
// Scenario: BUG-146 fix unblocks the RPC-043 noop-build assertion
// ===========================================================================

#[test]
fn scenario_bug146_unblocks_rpc043_noop_build() {
    // @step Given the fix has been applied to rust/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step And RPC-043 (the 7-module split of rust/napi/src/session_manager.rs) has NOT yet landed
    // (no assertion — works either way)

    // @step When I run `cargo build -p codelet-napi --features noop`
    let out = run_cargo(&["build", "-p", "codelet-napi", "--features", "noop"]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // @step Then the failure mode is no longer "cannot find attribute `napi` in this scope"
    assert!(
        !stderr.contains("cannot find attribute `napi` in this scope"),
        "BUG-146: stderr must NOT contain `cannot find attribute napi in this scope`; stderr tail:\n{}",
        tail(&stderr, 4000),
    );

    // @step And the failure mode is either success or an RPC-043 structural error (not an rpc-types attribute error)
    if !out.status.success() {
        // If it fails, the failure must NOT be an rpc-types attribute error.
        assert!(
            !stderr.contains("rpc-types/src/lib.rs") || !stderr.contains("attribute"),
            "BUG-146: post-fix build failure must NOT be an rpc-types attribute error; stderr tail:\n{}",
            tail(&stderr, 4000),
        );
    }
}
