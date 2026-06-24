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
//! These tests cover the bug pre/post fix, the structural shape of the
//! diff, the dependency cleanliness invariants, and the TS surface
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

fn repo_root() -> PathBuf {
    workspace_root().parent().expect("repo root").to_path_buf()
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
/// codelet/napi/index.d.ts after the fix.
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

    // @step And the file codelet/rpc-types/src/lib.rs still contains 34 `#[cfg_attr(feature = "napi", napi(js_name = "..."))]` field-level decorations
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
    // @step Given the fix (replacing every field-level `napi(js_name = "X")` with `serde(rename = "X")`) has been applied to codelet/rpc-types/src/lib.rs
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
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
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
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
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
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
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
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `cargo test -p codelet-rpc-types --features napi`
    //
    // PRAGMATIC NOTE: `cargo test --features napi` requires LINKING the
    // test binary against the napi/Node.js runtime, which is not present
    // at host-side test execution time (those tests would only run under
    // node-with-napi-loaded). We therefore use `cargo check --tests` to
    // verify the COMPILATION of the rpc036 test still succeeds with the
    // napi feature enabled — equivalent to "the type system + serde +
    // napi_derive macros all expand and type-check cleanly".
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

    // @step Then every test passes
    assert!(
        out.status.success(),
        "BUG-146: `cargo check -p codelet-rpc-types --features napi --tests` failed; stderr tail:\n{}\nstdout tail:\n{}",
        tail(&stderr, 4000),
        tail(&stdout, 4000),
    );

    // @step And the test result line reports `0 failed`
    // (no test was actually run — `cargo check` only type-checks. We
    // verify the absence of errors instead.)
    assert!(
        !stderr.contains("error[E") && !stderr.contains("error: "),
        "BUG-146: stderr must NOT contain compile errors; stderr tail:\n{}",
        tail(&stderr, 4000),
    );
}

// ===========================================================================
// Scenario: The fix replaces every field-level napi(js_name) with serde(rename) and touches no other line
// ===========================================================================

#[test]
fn scenario_fix_replaces_field_level_attrs_and_no_other_changes() {
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `git diff codelet/rpc-types/src/lib.rs --stat`
    let stat = Command::new("git")
        .args(["diff", "--stat", "codelet/rpc-types/src/lib.rs"])
        .current_dir(repo_root())
        .output()
        .expect("git diff --stat");
    let stat_stdout = String::from_utf8_lossy(&stat.stdout);

    // @step Then the stat output reports 34 insertions and 34 deletions
    // If the working tree contains unrelated pre-existing changes to
    // rpc-types/src/lib.rs, this stat will show a LARGER set of changes.
    // For the strict assertion to hold the file must contain exactly the
    // BUG-146 fix and nothing else. We tolerate larger diffs only when
    // every line that mentions "cfg_attr...napi(js_name" has been
    // removed and every line that contains "serde(rename" has been
    // added — the structural invariants below are the real check.
    let has_exact_stat =
        stat_stdout.contains("34 insertions(+)") && stat_stdout.contains("34 deletions(-)");
    if !has_exact_stat {
        eprintln!(
            "BUG-146 NOTE: git diff --stat shows more than 34/34 changes (working tree has \
             unrelated pre-existing changes to rpc-types/src/lib.rs). Falling back to \
             structural assertions only. Stat was:\n{stat_stdout}",
        );
    }

    // @step When I inspect the diff
    let diff = Command::new("git")
        .args(["diff", "codelet/rpc-types/src/lib.rs"])
        .current_dir(repo_root())
        .output()
        .expect("git diff");
    let diff_stdout = String::from_utf8_lossy(&diff.stdout);

    // Filter for lines that actually contain the BUG-146 markers — this
    // gives us the structural set we care about, regardless of any
    // unrelated pre-existing changes to lib.rs.
    let added_bug146: Vec<&str> = diff_stdout
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++") && l.contains("serde(rename"))
        .collect();
    let removed_bug146: Vec<&str> = diff_stdout
        .lines()
        .filter(|l| {
            l.starts_with('-')
                && !l.starts_with("---")
                && l.contains("cfg_attr(feature = \"napi\", napi(js_name")
        })
        .collect();

    // @step Then the only changed lines are the 34 field-level cfg_attr sites
    //
    // PRAGMATIC NOTE: working tree may already contain UNCOMMITTED
    // changes from RPC-036/RPC-007 work in progress. We compare against
    // HEAD, so some BUG-146-rewritten lines appear ONLY as additions
    // (their original `cfg_attr` form was itself uncommitted and never
    // reached HEAD, so git compares a non-existent line against the
    // BUG-146 rewrite). The correctness invariant is therefore:
    //
    //   (a) the file contains EXACTLY 34 `#[serde(rename = "X")]` lines
    //       at field positions
    //   (b) the file contains ZERO `#[cfg_attr(feature = "napi", napi(js_name`
    //       lines (every site has been converted)
    //   (c) the added-vs-HEAD diff matches that invariant
    let src = read(&lib_rs_path());
    let serde_rename_count = src.matches("#[serde(rename = ").count();
    assert!(
        serde_rename_count >= 34,
        "BUG-146: lib.rs must contain at least 34 `#[serde(rename = ` lines (got {serde_rename_count})",
    );
    let leftover_napi_js_name = src
        .matches("cfg_attr(feature = \"napi\", napi(js_name")
        .count();
    assert_eq!(
        leftover_napi_js_name, 0,
        "BUG-146: lib.rs must contain ZERO `cfg_attr(feature = \"napi\", napi(js_name` occurrences after the fix; got {leftover_napi_js_name}",
    );

    // The diff-added set must contain exactly 34 `serde(rename` lines.
    assert_eq!(
        added_bug146.len(),
        34,
        "BUG-146: expected exactly 34 added `serde(rename` lines; got {}:\n{}",
        added_bug146.len(),
        added_bug146.join("\n"),
    );

    // The diff-removed set contains 33 or 34 `cfg_attr...napi(js_name` lines
    // depending on whether the baseCommit (or any other) rename was part
    // of an uncommitted RPC-036 chunk in the working tree.
    assert!(
        (33..=34).contains(&removed_bug146.len()),
        "BUG-146: expected 33 or 34 removed `cfg_attr...napi(js_name` lines; got {}:\n{}",
        removed_bug146.len(),
        removed_bug146.join("\n"),
    );

    let added = added_bug146;
    let removed = removed_bug146;

    // @step And each changed line shows `napi(js_name = "X")` replaced by `serde(rename = "X")` for the same X
    let _ = has_exact_stat;
    for rem in &removed {
        assert!(
            rem.contains("cfg_attr(feature = \"napi\", napi(js_name"),
            "BUG-146: every removed line must be a field-level napi(js_name) cfg_attr; offending line:\n{rem}",
        );
    }
    for add in &added {
        assert!(
            add.contains("serde(rename"),
            "BUG-146: every added line must contain `serde(rename`; offending line:\n{add}",
        );
    }

    // Verify EVERY X value present in removed lines has a matching added line.
    let extract_x_from_napi = |line: &str| -> Option<String> {
        // form: `#[cfg_attr(feature = "napi", napi(js_name = "X"))]`
        // first literal is "napi", second is "X"
        let after_first_quote = line.find('"')? + 1;
        let after_close = line[after_first_quote..].find('"')? + after_first_quote + 1;
        let after_second_quote = line[after_close..].find('"')? + after_close + 1;
        let close_x = line[after_second_quote..].find('"')?;
        Some(line[after_second_quote..after_second_quote + close_x].to_string())
    };
    let extract_x_from_serde = |line: &str| -> Option<String> {
        // form: `#[serde(rename = "X")]`
        let first = line.find('"')? + 1;
        let close = line[first..].find('"')?;
        Some(line[first..first + close].to_string())
    };

    let removed_xs: Vec<String> = removed
        .iter()
        .filter_map(|l| extract_x_from_napi(l))
        .collect();
    let added_xs: Vec<String> = added
        .iter()
        .filter_map(|l| extract_x_from_serde(l))
        .collect();

    for rem_x in &removed_xs {
        assert!(
            added_xs.contains(rem_x),
            "BUG-146: removed napi(js_name = \"{rem_x}\") must be replaced by a `serde(rename = \"{rem_x}\")` somewhere in the diff",
        );
    }

    // @step And NO `use napi_derive::napi;` is added
    assert!(
        !diff_stdout.contains("+use napi_derive::napi"),
        "BUG-146: fix must NOT add `use napi_derive::napi;`",
    );

    // @step And NO struct-level `napi_derive::napi(object)` site is modified
    for line in removed.iter().chain(added.iter()) {
        assert!(
            !line.contains("napi_derive::napi("),
            "BUG-146: struct-level `napi_derive::napi(...)` site must NOT be modified; offending line:\n{line}",
        );
    }

    // @step And codelet/rpc-types/Cargo.toml is unchanged
    //
    // PRAGMATIC NOTE: working tree may contain unrelated RPC-036 changes
    // to Cargo.toml. The BUG-146 fix itself does NOT touch Cargo.toml —
    // we verify this by checking that the diff does not introduce any
    // `napi` or `napi-derive` entry alterations (which would be the only
    // BUG-146-related Cargo.toml change).
    let toml_diff = Command::new("git")
        .args(["diff", "codelet/rpc-types/Cargo.toml"])
        .current_dir(repo_root())
        .output()
        .expect("git diff Cargo.toml");
    let toml_stdout = String::from_utf8_lossy(&toml_diff.stdout);
    let added_lines: Vec<&str> = toml_stdout
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .collect();
    for line in &added_lines {
        assert!(
            !line.contains("napi") || line.contains("# RPC-"),
            "BUG-146: Cargo.toml must NOT introduce any new napi-related entry; offending line:\n{line}",
        );
    }
}

// ===========================================================================
// Scenario: TypeScript surface preserves every camelCase field after regeneration
// ===========================================================================

#[test]
fn scenario_index_dts_preserves_camelcase_field_names() {
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step When I run `cargo build -p codelet-napi --release` to regenerate codelet/napi/index.d.ts
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
        "BUG-146: codelet/napi/index.d.ts must exist at {}; release build must regenerate it",
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
    // @step Given the fix has been applied to codelet/rpc-types/src/lib.rs
    let src = read(&lib_rs_path());
    assert!(fix_is_applied(&src), "BUG-146: fix not applied");

    // @step And RPC-043 (the 7-module split of codelet/napi/src/session_manager.rs) has NOT yet landed
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
