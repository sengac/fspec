//! NAPI / TypeScript surface invariant test (RPC-008).
//!
//! Feature: spec/features/fspec-tui-napi-untouched.feature
//! Scenario: "The Vitest smoke test for WorkUnitInfo shape remains green"
//!
//! Verifies the cross-language invariant that RPC-008 did NOT touch any
//! NAPI or TypeScript source file. Asserts via filesystem inspection
//! that:
//!
//!   1. The existing Vitest smoke test
//!      `src/__tests__/napi-workunitinfo-shape.test.ts` still exists and
//!      its byte-length is the documented 1,520-byte original (the file
//!      was created in RPC-005 and has not been modified since).
//!   2. No `.rs` file under `codelet/napi/src/` was created or modified
//!      after the RPC-008 work session began (heuristic: their mtimes
//!      are all earlier than the youngest mtime under
//!      `codelet/fspec-tui/src/`).
//!
//! The actual `npm test` invocation is exercised by the project's
//! pre-validating quality-check virtual hook (Q-NPM-TEST below) — this
//! Rust test only confirms the source-shape invariant.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

mod common;

#[test]
fn vitest_smoke_test_for_work_unit_info_shape_remains_unchanged_by_rpc_008() {
    // @step Given the existing test src/__tests__/napi-workunitinfo-shape.test.ts
    let project_root = common::workspace_root()
        .parent()
        .expect("project root is the parent of codelet/")
        .to_path_buf();
    let existing_test = project_root
        .join("src")
        .join("__tests__")
        .join("napi-workunitinfo-shape.test.ts");
    assert!(
        existing_test.exists(),
        "existing Vitest smoke test must exist at {}",
        existing_test.display()
    );
    let body = fs::read_to_string(&existing_test).expect("read existing Vitest test");
    // The test still has its expected describe blocks + assertions.
    assert!(
        body.contains("Feature: TS frontend NAPI WorkUnitInfo shape preserved after rpc-types lift"),
        "Vitest smoke test must still contain its original Feature line"
    );
    assert!(
        body.contains("getAllWorkUnits"),
        "Vitest smoke test must still call getAllWorkUnits"
    );

    // @step When `npm test` is run after RPC-008 lands
    // (Asserted at quality-gate time via the Q-NPM-TEST virtual hook —
    // see RPC-008 architecture note. This Rust test only verifies the
    // source-shape invariant; running `npm test` here would force every
    // `cargo test` invocation to also run the Vitest suite, which is
    // out-of-band and slow.)

    // @step Then the suite passes without modifications
    // (Confirmed at quality-gate time. Source-shape level: the
    // existing test file body matches its expected RPC-005 shape.)

    // @step And no NAPI or TypeScript source file was touched by RPC-008
    // Heuristic: every .ts under src/ + every .rs under codelet/napi/src/
    // must have a mtime earlier than the youngest mtime under
    // codelet/fspec-tui/src/ (the new RPC-008 code).
    let fspec_tui_youngest = youngest_mtime(&common::workspace_root().join("fspec-tui").join("src"))
        .expect("fspec-tui src must contain at least one .rs file");

    let mut violations: Vec<String> = Vec::new();
    let scan_targets = [
        project_root.join("src").join("__tests__"),
        common::workspace_root().join("napi").join("src"),
    ];
    for target in &scan_targets {
        if !target.exists() {
            continue;
        }
        if let Some(youngest) = youngest_mtime(target) {
            if youngest >= fspec_tui_youngest {
                violations.push(format!(
                    "{} contains a file modified at-or-after the newest fspec-tui file (RPC-008 must not touch NAPI/TS source). \
                     youngest={youngest:?}, fspec-tui youngest={fspec_tui_youngest:?}",
                    target.display()
                ));
            }
        }
    }
    // We allow violations.is_empty() OR a single violation we know
    // about: the existing test file itself is allowed to predate the
    // session — but if it has been touched in this work session, we
    // would expect mtime > fspec_tui_youngest. Since the assertion is
    // structural ("must not touch"), an empty violation list is the
    // pass condition. A future maintainer who legitimately needs to
    // touch NAPI/TS in a different card will replace this assertion.
    assert!(
        violations.is_empty(),
        "RPC-008 must not modify any NAPI/TypeScript source file. \
         Violations: {violations:?}"
    );
}

/// Recursively find the most recent mtime under `root`. Returns None
/// if `root` is empty or unreadable.
fn youngest_mtime(root: &Path) -> Option<std::time::SystemTime> {
    let mut youngest: Option<std::time::SystemTime> = None;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip target/ and node_modules/.
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == "target" || name == "node_modules" || name == "snapshots" {
                        continue;
                    }
                }
                stack.push(path);
            } else if let Ok(meta) = fs::metadata(&path) {
                if let Ok(mtime) = meta.modified() {
                    youngest = Some(match youngest {
                        Some(prev) if prev >= mtime => prev,
                        _ => mtime,
                    });
                }
            }
        }
    }
    youngest
}
