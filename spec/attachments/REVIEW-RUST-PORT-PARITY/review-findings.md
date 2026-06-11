# Epic Review: 20 Rust-Ported CLI Commands — Parity Analysis & Fixes

**Date:** 2026-06-11
**Reviewer:** Claude Code (supervisor) + 4 parallel review agents + 1 cargo-serial worker
**Scope:** All 20 uncommitted Rust ports across batches 8 & 9

## Commands Reviewed

| # | RPC | Command | Status |
|---|-----|---------|--------|
| 1 | RPC-168 | add-architecture-note | ✅ PASS |
| 2 | RPC-169 | add-assumption | ✅ PASS |
| 3 | RPC-177 | add-dependency | ✅ PASS |
| 4 | RPC-181 | add-example | ✅ PASS |
| 5 | RPC-188 | add-question | 🔧 FIXED |
| 6 | RPC-189 | add-rule | ✅ PASS |
| 7 | RPC-193 | add-tag-to-feature | 🔧 FIXED (critical) |
| 8 | RPC-194 | add-tag-to-scenario | 🔧 FIXED |
| 9 | RPC-196 | answer-question | ✅ PASS |
| 10 | RPC-267 | remove-architecture-note | 🔧 FIXED |
| 11 | RPC-273 | remove-example | ✅ PASS |
| 12 | RPC-278 | remove-question | 🔧 FIXED |
| 13 | RPC-279 | remove-rule | ✅ PASS |
| 14 | RPC-281 | remove-tag-from-feature | ✅ PASS |
| 15 | RPC-282 | remove-tag-from-scenario | 🔧 FIXED |
| 16 | RPC-287 | restore-architecture-note | ✅ PASS |
| 17 | RPC-289 | restore-example | ✅ PASS |
| 18 | RPC-290 | restore-question | 🔧 FIXED |
| 19 | RPC-291 | restore-rule | ✅ PASS |
| 20 | RPC-298 | set-user-story | ✅ PASS |

## Issues Found & Fixed

### 🔴 Critical Issue #1 — `add-tag-to-feature` tag insertion ordering
**File:** `codelet/fspec-core/src/commands/add_tag_to_feature.rs:327-353`

**Bug:** When a feature file has exactly one existing tag immediately above the `Feature:` line (e.g. `@existing-tag\nFeature:`), the Rust port placed the new tag AFTER the existing tag, whereas TS places it BEFORE. The Rust port was missing the `i == 0 → insert_at = 0` clamp at the end of the backward walk.

**TS reference:** `src/commands/add-tag-to-feature.ts:174-177`
**Fix applied:** Added the missing `if i == 0 { insert_at = 0; break; }` clamp inside the backward walk.

**Collateral fixes:**
- Updated test assertion in `codelet/fspec-core/tests/add_tag_to_feature.rs:258-286` (was encoding the buggy behaviour as expected) to match TS-actual ordering.
- Updated feature file `spec/features/add-tag-to-feature-rust-port.feature:80-84` to document the corrected ordering.

**Verification:** Parity test against TS now produces byte-identical feature files.

### 🟡 Warning #2 — `add-question` stderr prefix divergence
**File:** `codelet/fspec/src/add_question.rs:65`

**Bug:** Rust bridge emitted `Error:` on failure; TS emits `✗ Failed to add question:` (`src/commands/add-question.ts:97`).

**Fix:** Changed bridge stderr prefix to `✗ Failed to add question:`. Updated doc-string comment, feature file `spec/features/add-question-cli-subcommand.feature` (scenarios 4 and 5), CLI test assertions, and coverage file scenario names.

### 🟡 Warning #3 — `remove-question` stderr prefix + NaN handling
**Files:** `codelet/fspec/src/remove_question.rs`, `codelet/fspec/src/main.rs:710`, `codelet/fspec-core/src/commands/remove_question.rs:33-100`

**Bugs:**
- Bridge stderr prefix `Error:` vs TS `✗ Failed to remove question:`
- Non-numeric `--index abc` rejected by clap with exit 2 instead of flowing through TS NaN path to "Question with ID NaN not found" (exit 1).

**Fix:**
- Changed clap variant `INDEX` from `u64` → `String`.
- Added `parse_ts_int_radix10` shim in bridge (mirrors `remove_rule`/`remove_example` pattern).
- Replaced core's `index: u64` with the `TsIndex { Int(i64), Nan }` enum + `deserialize_ts_index` deserializer (already proven pattern from `remove_rule`).
- Updated bridge stderr prefix to `✗ Failed to remove question:`.
- Updated `spec/features/remove-question-cli-subcommand.feature` (scenarios 4 and 5) + coverage file + CLI test assertions.

### 🟡 Warning #4 — `remove-architecture-note` NaN handling
**Files:** `codelet/fspec/src/remove_architecture_note.rs`, `codelet/fspec/src/main.rs:732`, `codelet/fspec-core/src/commands/remove_architecture_note.rs:35-105`

**Bug:** Non-numeric `<index>` rejected by clap (exit 2) instead of flowing through TS NaN path to `Architecture note with ID NaN not found` (exit 1).

**Fix:**
- Changed clap variant `INDEX` from `u64` → `String`.
- Added `parse_ts_int_radix10` shim in bridge.
- Replaced core's `index: u64` with `TsIndex` enum + `deserialize_ts_index`.

### 🟡 Warning #5 — `restore-question` stderr prefix divergence
**File:** `codelet/fspec/src/restore_question.rs:64`

**Fix:** Bridge stderr prefix `Error:` → `✗ Failed to restore question:` (TS at `src/commands/restore-question.ts:107`). Updated doc-string, feature file, test scenario names + coverage file.

### 🟡 Warning #6/#7 — `add-tag-to-scenario` / `remove-tag-from-scenario` scope drift
**Files:** `codelet/fspec-core/src/commands/add_tag_to_scenario.rs:307-324`, `codelet/fspec-core/src/commands/remove_tag_from_scenario.rs:212-232`

**Bug:** `find_scenario` walked both top-level `feature.scenarios` AND `feature.rules[*].scenarios[*]`, but TS only walks `gherkinDocument.feature.children.filter(c => c.scenario)` which is top-level only. This broadened the Rust behaviour beyond TS.

**Fix:** Removed the rule-walk in both files; updated comments to document strict TS parity.

**Verification:** TS and Rust now produce identical "Scenario not found" error when target is inside a `Rule:` block.

### 🔧 Test Infrastructure — `cargo_shape.rs` locked file list
**File:** `codelet/fspec/tests/cargo_shape.rs:334-540`

The locked-file invariant test required updating to include the 20 new bridge files. Also bumped `main_cap` from 1100 → 1500 (main.rs grew to 1420 lines with 20 new clap variants + intercept arms + forward! arms) and `common_cap` from 800 → 900 (existing growth).

## Out-of-Scope Issues (Cross-Cutting, Pre-Existing)

These were flagged by reviewers but deferred because they affect the entire port architecture, not the 20 newly-ported commands specifically:

1. **JSON key-order divergence in `WorkUnitsData`** — Rust `serde` derive serializes struct fields in declaration order (`version → meta → workUnits → states → extra-tail`), whereas TS preserves arbitrary insertion order via `Object.assign`. This is byte-noise only — functionally lossless, no data dropped. Affects ALL work-units.json mutators across the port. A future RPC card should switch the top-level structure to `serde_json::Value` / `IndexMap` to preserve arbitrary key order.

2. **`.expect("present")` invariants in production code** — Production code in several ported commands uses `.expect(...)` after explicit prior `contains_key`/`is_empty` guards. The expects are not user-reachable, but `clippy::expect_used` is only allowed in `#[cfg(test)]` blocks. A future cleanup card could refactor to `match`/`?` for strict "no panics in production" hygiene.

## Verification Results

After fixes, all 20 + cross-cutting tests pass:

| Test Suite | Result |
|------------|--------|
| 20 dispatcher tests (`cargo test -p codelet-fspec-core --test <cmd>`) | 100% PASS |
| 20 CLI shell tests (`cargo test -p codelet-fspec --test cli_<cmd>`) | 100% PASS |
| `cargo_shape` (locked file layout + size caps) | ✅ PASS |
| `dispatcher_test` (shared dispatcher invariants) | ✅ PASS |
| `cross_frontend_parity` (two-front-doors invariant) | ✅ PASS |
| Direct TS-vs-Rust parity probes (7 manually-crafted scenarios) | 7/7 byte-equal |

## Files Modified

### Core impl
- `codelet/fspec-core/src/commands/add_tag_to_feature.rs` (insertion clamp fix)
- `codelet/fspec-core/src/commands/add_tag_to_scenario.rs` (rule-walk removal)
- `codelet/fspec-core/src/commands/remove_tag_from_scenario.rs` (rule-walk removal)
- `codelet/fspec-core/src/commands/remove_question.rs` (TsIndex NaN support)
- `codelet/fspec-core/src/commands/remove_architecture_note.rs` (TsIndex NaN support)

### CLI bridges
- `codelet/fspec/src/add_question.rs` (Failed prefix)
- `codelet/fspec/src/remove_question.rs` (rewrite: NaN shim + Failed prefix)
- `codelet/fspec/src/remove_architecture_note.rs` (rewrite: NaN shim)
- `codelet/fspec/src/restore_question.rs` (Failed prefix)

### Shared
- `codelet/fspec/src/main.rs` (RemoveQuestion + RemoveArchitectureNote clap variants now take `String` index)

### Tests
- `codelet/fspec-core/tests/add_tag_to_feature.rs` (test assertion realigned with TS)
- `codelet/fspec/tests/cli_add_question.rs` (Failed prefix assertions)
- `codelet/fspec/tests/cli_remove_question.rs` (Failed prefix assertions)
- `codelet/fspec/tests/cli_restore_question.rs` (scenario name realignment)
- `codelet/fspec/tests/cargo_shape.rs` (locked file list + caps bumped for 20 new bridges)

### Feature files (specifications)
- `spec/features/add-tag-to-feature-rust-port.feature`
- `spec/features/add-question-cli-subcommand.feature`
- `spec/features/remove-question-cli-subcommand.feature`
- `spec/features/restore-question-cli-subcommand.feature`

### Coverage files (auto-updated to match scenario name changes)
- `spec/features/add-question-cli-subcommand.feature.coverage`
- `spec/features/remove-question-cli-subcommand.feature.coverage`
- `spec/features/restore-question-cli-subcommand.feature.coverage`

## Final Verdict

All 20 newly ported Rust CLI commands now exhibit byte-exact TS parity for the surfaces tested. All affected cargo test suites pass. The 4 worker reviews + cargo serial worker pattern from `command-port.md §13` proved effective for parallelizing parity analysis across the batch.
