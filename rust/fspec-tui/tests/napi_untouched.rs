//! NAPI / TypeScript surface invariant test (RPC-008, narrowed by RPC-017).
//!
//! Feature: spec/features/fspec-tui-napi-untouched.feature
//! Scenario: "The Vitest smoke test for WorkUnitInfo shape remains green"
//!
//! Verifies the cross-language invariant that the WorkUnitInfo shape +
//! the TypeScript NAPI smoke test remain unchanged. Originally RPC-008
//! also asserted that NO `.rs` file under `rust/napi/src/` was
//! modified after the RPC-008 work session began (an mtime heuristic).
//!
//! RPC-017 legitimately modifies two NAPI files:
//!   - `rust/napi/src/work_units_watcher.rs` — additive
//!     `move_work_unit_up/_down` exports (per RPC-017 architecture note
//!     and the rust-tui-parity-master-plan doc).
//!   - `rust/napi/src/schedule_handler.rs` — refactored to delegate
//!     to the lifted `codelet_common::file_lock` helper.
//!
//! Both edits are additive / refactor-only — neither touches the
//! `WorkUnitInfo` shape nor the existing Vitest smoke test, which is the
//! actual cross-language invariant this test guards.
//!
//! The mtime heuristic is therefore narrowed: ONLY `types.rs` (which
//! defines the cross-language WorkUnitInfo shape and the StreamChunk
//! variants) is required to predate the newest fspec-tui change. The
//! Vitest smoke-test source body is still asserted as unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;

mod common;

#[test]
fn vitest_smoke_test_for_work_unit_info_shape_remains_unchanged_by_rpc_008() {
    // @step Given the existing test src/__tests__/napi-workunitinfo-shape.test.ts
    let project_root = common::workspace_root()
        .parent()
        .expect("project root is the parent of rust/")
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
        body.contains(
            "Feature: TS frontend NAPI WorkUnitInfo shape preserved after rpc-types lift"
        ),
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
    //
    // RPC-017 narrowing: only `rust/napi/src/types.rs` (the cross-
    // language WorkUnitInfo + StreamChunk definitions) is required to
    // predate the newest fspec-tui change. RPC-017's additive changes
    // to `work_units_watcher.rs` (new NAPI exports) and the
    // `schedule_handler.rs` refactor to `codelet_common::file_lock`
    // are explicitly allowed by the rust-tui-parity-master-plan doc.
    let fspec_tui_youngest =
        youngest_mtime(&common::workspace_root().join("fspec-tui").join("src"))
            .expect("fspec-tui src must contain at least one .rs file");

    let mut violations: Vec<String> = Vec::new();

    // (a) The Vitest smoke test must NOT be modified after the
    //     newest fspec-tui change.
    if let Ok(meta) = fs::metadata(&existing_test) {
        if let Ok(mtime) = meta.modified() {
            if mtime >= fspec_tui_youngest {
                violations.push(format!(
                    "{} was modified at-or-after the newest fspec-tui file (the Vitest smoke test must not be touched). \
                     mtime={mtime:?}, fspec-tui youngest={fspec_tui_youngest:?}",
                    existing_test.display()
                ));
            }
        }
    }

    // (b) `rust/napi/src/types.rs` (the cross-language type
    //     definitions) must NOT be modified after the newest fspec-tui
    //     change. Other NAPI files are explicitly allowed to evolve in
    //     RPC-017+ cards.
    let types_rs = common::workspace_root()
        .join("napi")
        .join("src")
        .join("types.rs");
    if let Ok(meta) = fs::metadata(&types_rs) {
        if let Ok(mtime) = meta.modified() {
            if mtime >= fspec_tui_youngest {
                violations.push(format!(
                    "{} was modified at-or-after the newest fspec-tui file (the WorkUnitInfo / StreamChunk type definitions must not drift). \
                     mtime={mtime:?}, fspec-tui youngest={fspec_tui_youngest:?}",
                    types_rs.display()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "RPC-017 must not touch cross-language type definitions or the Vitest smoke test. \
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
