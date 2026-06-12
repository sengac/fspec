# Batch 12 — TS→Rust Parity Analysis & Fix Report

**Date:** 2026-06-12
**Reviewer:** Claude Code (supervisor) + 4 parallel parity workers + 1 cargo serial worker
**Scope:** The 10 most-recently ported RPC commands (Batch 12)
**Method:** Ran the original TypeScript CLI (`fspec` → `/home/rquast/projects/fspec.orig/dist/index.js` v0.9.3) against the Rust port (`codelet/target/release/fspec`) in isolated temp dirs; diffed stdout, stderr, exit codes, and on-disk `spec/*.json` side effects. All cargo/npm work serialized through one dedicated agent.

## Commands Reviewed

| RPC | Command | Initial verdict | Final |
|-----|---------|-----------------|-------|
| RPC-223 | delete-work-unit | FAIL (3) | ✅ PASS |
| RPC-227 | export-dependencies | PASS | ✅ PASS |
| RPC-228 | export-example-map | FAIL (1) | ✅ PASS |
| RPC-206 | compact-work-unit | FAIL (2) | ✅ PASS |
| RPC-264 | record-iteration | PASS | ✅ PASS |
| RPC-229 | export-work-units | PASS | ✅ PASS |
| RPC-317 | update-work-unit | FAIL (2) | ✅ PASS |
| RPC-318 | update-work-unit-estimate | FAIL (4) | ✅ PASS |
| RPC-255 | prioritize-work-unit | FAIL (3) | ✅ PASS |
| RPC-284 | repair-work-units | PASS | ✅ PASS |

## Divergences Found & Fixed

### 1. Top-level key ordering of `spec/work-units.json` not preserved (CRITICAL, cross-cutting)
Affected delete / compact / prioritize / repair (every command that re-serialises the typed `WorkUnitsData`). TS round-trips on-disk key order verbatim (real files read `meta, migrationHistory, prefixCounters, states, version, workUnits`); Rust emitted the fixed struct order. **Fix:** manual order-preserving `Serialize`/`Deserialize` on `WorkUnitsData` capturing on-disk `field_order` (mirrors the existing per-`WorkUnit` approach). `codelet/fspec-core/src/types/work_unit.rs`.

### 2. Nested `meta` block key ordering (CRITICAL, surfaced during fix verification)
Real files store `meta` as `{ lastUpdated, version }`; the `Meta` struct re-emitted `{ version, lastUpdated }`. **Fix:** manual order-preserving (de)serialisation on `Meta` with `field_order` + `extra`. Same file.

### 3. `--position` numeric coercion used strict `i64::parse` instead of JS `parseInt` (CRITICAL — prioritize)
`12abc`/`2x`/`3.7`/`0x10`/`-1` all diverged (wrong ordering, wrong exit code). **Fix:** CLI bridge now uses `parse_js_int`; added `allow_hyphen_values=true` so `-1` reaches the bridge instead of being rejected by clap. `codelet/fspec/src/prioritize_work_unit.rs`, `codelet/fspec/src/main.rs`.

### 4. `<estimate>` coercion + `NaN` rendering (CRITICAL — update-work-unit-estimate)
TS `parseInt` accepts `5abc`→5 / `13.9`→13 and renders `Invalid estimate: NaN` for non-numeric; Rust rejected them and printed a garbage `i64::MIN` sentinel. **Fix:** bridge uses `parse_js_int`; core accepts a raw `Value` estimate and renders `NaN` via `js_estimate_display`. `codelet/fspec/src/update_work_unit_estimate.rs`, `codelet/fspec-core/src/commands/update_work_unit_estimate.rs`.

### 5. Extra blank line in both ACDD-violation messages (CRITICAL — update-work-unit-estimate)
Two `String::new()` produced two blank lines after `</system-reminder>`; TS has one. **Fix:** removed one blank in both `no_feature_file_message` and `prefill_placeholders_message`.

### 6. `--force` accepted by Rust but rejected by TS (CRITICAL — compact-work-unit)
TS Commander registers only `<workUnitId>` (no `--force`), so `--force` → `unknown option`; Rust accepted and honoured it. The TS *help text* still advertises `--force` (a TS quirk). **Fix:** removed the `force` clap arg (help config unchanged → `--help` stays byte-identical, `--force` now errors); bridge always passes `force:false`. `codelet/fspec/src/main.rs`, `codelet/fspec/src/compact_work_unit.rs`.

### 7. Missing same-epic / same-parent array reorder (CRITICAL — update-work-unit)
Rust added `if old != new` guards that TS lacks; TS removes-then-re-appends on same-epic/same-parent reassignment (moves id to END of the array). **Fix:** removed both guards to restore bug-for-bug parity. `codelet/fspec-core/src/commands/update_work_unit.rs`.

### 8. Missing-required-argument name lowercased (delete/export-example-map/etc.)
`'workUnitId'` (TS) vs `'workunitid'` (Rust): `commander_arg_name` lowercased already-camelCase `value_name`s. **Fix:** preserve mixed-case tokens verbatim; only convert UPPER_SNAKE and lowercase single all-caps words. `codelet/fspec/src/main.rs`.

### 9. "too many arguments" pluralisation (delete-work-unit/etc.)
Rust always said "Expected 1 arguments"; Commander uses singular for 1. **Fix:** pluralise the expected noun. `codelet/fspec/src/main.rs`.

### 10. Pre-existing `unused variable: lines` warning
Removed the dead `lines` parameter from `push_tag_matches`. `codelet/fspec-core/src/commands/update_work_unit_estimate.rs`.

## Verification

- Release binary rebuilt clean (0 warnings, 0 errors).
- All affected test suites pass: core lib **452 passed / 0 failed**; 12 core dispatcher test binaries **0 failed**; 10 CLI integration test binaries **0 failed**.
- Empirical re-comparison vs TS: ordering byte-identical for prioritize/delete/compact/repair; all `--position`/`<estimate>` quirk values match; same-epic reorder matches; error texts match; `--help` byte-identical for all 10; 20/20 misc parity checks pass.

## Notes
- Non-issue: `meta.lastUpdated` precision on the empty auto-create path — `iso8601_now()` already emits millisecond precision; any observed `.000Z` was a wall-clock coincidence between the two separate runs.
- `record-iteration` and `export-work-units` preserve the broken TS CLI shell behaviour bug-for-bug (already correct).

---

## SUPERVISOR RECONCILIATION (post re-verification — turn 268+)

**Trigger:** Re-check workers w2 (compact-work-unit) and w3 (update-work-unit, update-work-unit-estimate) reported these commands STILL FAIL after the first-pass fixes were claimed applied.

**Verdict: NO REGRESSIONS. All reported divergences are RESOLVED in the current release binary.**
The w2/w3 FAIL reports were based on a STALE binary / pre-fix observations. Source fixes ARE present and ARE compiled into `/home/rquast/projects/fspec/codelet/target/release/fspec` (built 21:14, source last modified ≤20:30).

### Ground-truth re-tests against the CURRENT binary (TS `fspec` v0.9.3 vs Rust port), seeded via `/tmp/parity-batch12/seed.sh`:

| Reported divergence | w-report | TS result | Rust result | Status |
|---|---|---|---|---|
| same-epic re-assign reorders epics.json workUnits | w3 #1 (crit) | `[AUTH-002, AUTH-001]` | `[AUTH-002, AUTH-001]` | ✅ MATCH (guard removed) |
| same-parent re-assign reorders children | w3 #2 (crit) | reorders | reorders | ✅ MATCH (guard removed) |
| estimate `5abc` parseInt → stores 5, echoes "5abc" | w3 #3 (crit) | stored=5, "set to 5abc" | stored=5, "set to 5abc" | ✅ MATCH |
| estimate `13.9` → accepted, "set to 13.9" | w3 #3 (crit) | accepted | accepted | ✅ MATCH |
| estimate `abc` → "Invalid estimate: NaN" | w3 #4 (crit) | "Invalid estimate: NaN" | "Invalid estimate: NaN" | ✅ MATCH |
| missing-arg casing `workUnitId` (not lowercased) | w3 #5 / fix#8 | `'workUnitId'` | `'workUnitId'` | ✅ MATCH |
| compact `--force` on non-done unit | w2 #1 (crit) | `error: unknown option '--force'` exit=1 | `error: unknown option '--force'` exit=1 | ✅ MATCH (clap arg removed) |
| compact done-unit work-units.json key order/bytes | w2 #2 (low) | order-preserved | **BYTE-IDENTICAL** | ✅ MATCH |

### Full regression sweep (`verify3.sh`): **pass=20 fail=0** (all 10 `--help` byte-identical + 10 runtime cases).

### Root-cause of the false FAILs:
- `release-build-final.txt` (21:14) = `Finished in 0.23s` (no-op). The binary-producing compile was `release-build-3.txt` (20:56, 11m31s) which compiled both `codelet-fspec-core` and the `codelet-fspec` CLI from the fixed source. Workers that ran before ~20:56 (or that transcribed pre-fix source-guard notes into their FAIL reports) observed the OLD behavior.
- Source confirms fixes present: `grep` for `if old != new_epic` / `if old != parent` in `update_work_unit.rs` → **no matches**; explicit bug-for-bug comments now document the no-guard behavior.

### Outstanding work: NONE for Batch 12. No further code changes required. Recommend re-running workers against the current binary only if additional confidence is desired; do NOT re-apply the "fixes" (already in place).

---

## SUPERVISOR RECONCILIATION — w1 (delete-work-unit RPC-223, export-dependencies RPC-227, export-example-map RPC-228)

**Same outcome as w2/w3: w1's 3 reported FAIL divergences are ALL RESOLVED in the current release binary.** w1 ran a stale (pre-20:56) binary.

| w1 divergence | TS | Rust (current) | Status |
|---|---|---|---|
| #1 (crit) delete-work-unit top-level key order | `[meta,migrationHistory,prefixCounters,states,version,workUnits]` | same — **BYTE-IDENTICAL** | ✅ FIXED (work_unit.rs order-preserving serialize) |
| #2 (minor) missing-arg casing `workUnitId` (delete & export-example-map) | `'workUnitId'` | `'workUnitId'` | ✅ FIXED (commander_arg_name) |
| #3 (minor) too-many-args singular "Expected 1 argument" | `Expected 1 argument but got 3.` | `Expected 1 argument but got 3.` | ✅ FIXED (pluralization) |
| export-example-map success JSON byte parity | identical | **BYTE-IDENTICAL** | ✅ |
| export-dependencies (RPC-227) | — | — | ✅ PASS (w1 confirmed) |

**Batch 12 status remains: 0 outstanding divergences.** w1's findings duplicate the same-class fixes (#1 key order, #8 arg casing, #9 pluralization) already verified for w2/w3. No further action.

---

## SUPERVISOR RECONCILIATION — w4 (prioritize-work-unit RPC-255, repair-work-units RPC-284)

**Same outcome as w1/w2/w3: w4's 3 prioritize-work-unit FAIL divergences are ALL RESOLVED in the current binary.** w4 ran a stale (pre-20:56) binary. repair-work-units already PASS.

| w4 divergence | TS | Rust (current) | Status |
|---|---|---|---|
| #1 (crit) parseInt prefix-parse `12abc`→12 / `2x`→2 / `3.7`→3 | reorders correctly | identical reorder | ✅ FIXED (bridge parse_js_int) |
| #2 (crit) `0x10`→0→"Invalid position: 0" exit | error, no mutation | error, no mutation | ✅ FIXED |
| #3 (med) `--position -1` → domain msg "Invalid position: -1" | domain error | domain error (not clap `unknown option`) | ✅ FIXED (allow_hyphen_values + parse_js_int) |
| full-file side effect after `--position 2` | — | **BYTE-IDENTICAL** | ✅ |

NOTE: all error cases exit 0 in BOTH (TS prioritize swallows error to exit 0) — bug-for-bug preserved.

**FINAL BATCH 12 STATUS: 0 outstanding divergences across all 10 commands / all 4 workers.** Every worker FAIL traced to a pre-rebuild stale binary; every flagged item verified RESOLVED + byte-identical against the current release binary (mtime 21:14).
