# Batch 4 Parity Review Report — Tag + Dependency Commands

**Reviewer:** Compliance + parity reviewer (subordinate of supervisor d43b8cb6-25ab-4c36-aa08-aefe1e6c7363)
**Date:** 2026-06-11
**Project:** fspec
**Scope:** RPC-177, RPC-193, RPC-281, RPC-194, RPC-282
**Cargo worker session:** 80efbebc-d7c7-4c2a-8c6b-d09e7fb240c5

Cargo tests for ALL 5 commands' dispatcher + CLI suites PASS (83 total tests, 0 failures). Help fixtures match byte-for-byte for all 5 commands. Functional parity is solid for `add-dependency`, `remove-tag-from-feature`, `add-tag-to-scenario`, and `remove-tag-from-scenario`. Two real divergences from TS were found — one in `add-tag-to-feature` (tag insertion ordering, user-visible) and one in `add-dependency` (JSON key order, byte-noise only).

---

## add-dependency (RPC-177)

### Summary: WARN

### 🔴 Critical Issues
None blocking shell parity (exit codes + stdout + stderr + side effects all match). See 🟡 for a structural divergence the supervisor should decide on.

### 🟡 Warnings

1. **JSON key ORDER & extra-field preservation diverges from TS.**
   The Rust port re-writes `spec/work-units.json` with `workUnits → states` then a `flatten`-tail (`prefixes`, `epics`) at the END.
   The TS impl preserves the original key order: `version → prefixes → epics → states → workUnits`.
   - File: `codelet/fspec-core/src/types/work_unit.rs` lines 1-15 — `WorkUnitsData` declares `version → meta → workUnits → states → #[serde(flatten)] extra` so any prior `prefixes/epics` is reshuffled to the tail.
   - Demonstrated by `diff` blocks at parity log lines 10-77, 87-156, 186-243 (`/tmp/parity-batch4-1781162580.txt`).
   - Functionally lossless (all fields preserved, no data dropped) but byte-noisy. If you compare two work-units.json from TS vs Rust they will differ even when semantically equal.

2. **Reliance on `.expect("source")` / `.expect("target")` / `.expect("work unit exists")` in hot path.**
   - `codelet/fspec-core/src/commands/add_dependency.rs:128, 142-146, 162, 176-180, 200, 220, 227, 275`.
   - All guarded by prior `contains_key`/`validate_target_exists`, so these are not user-reachable today, but they violate the "no `.expect()` without descriptive messages in production code" review criterion. The existing messages are minimally descriptive ("source"/"target"); acceptable per project conventions but worth a refactor to `.ok_or(FspecCoreError::Internal{...})?` if you want stricter ergonomics.

### Parity Matrix

| Test Case | TS Exit | TS stdout (excerpt) | Rust Exit | Rust stdout (excerpt) | Match? |
|-----------|---------|---------------------|-----------|-----------------------|--------|
| `add-dependency AUTH-002 AUTH-001` (shorthand) | 0 | `✓ Dependency added successfully` | 0 | `✓ Dependency added successfully` | ✅ stdout; ⚠️ JSON key-order |
| `add-dependency AUTH-001 --blocks API-001` | 0 | `✓ Dependency added successfully` | 0 | `✓ Dependency added successfully` | ✅ stdout; ⚠️ JSON key-order |
| `add-dependency AUTH-001 AUTH-001` (self-dep) | 1 | `✗ Failed to add dependency: Cannot create self-dependency` | 1 | same | ✅ |
| `add-dependency AUTH-001` (no rel) | 1 | `✗ Failed to add dependency: Must specify at least one relationship: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to` | 1 | same | ✅ |
| `add-dependency AUTH-001 AUTH-002 --depends-on API-001` (conflict) | 1 | `✗ Failed to add dependency: Cannot specify dependency both as argument and --depends-on option` | 1 | same | ✅ |
| `add-dependency AUTH-001 --relates-to API-001` | 0 | `✓ Dependency added successfully` | 0 | same | ✅ stdout; ⚠️ JSON key-order |
| `add-dependency --help` (vs `node dist/index.js add-dependency --help`) | 0 | full help text | 0 | identical | ✅ byte-exact |

### Files Reviewed
- `codelet/fspec-core/src/commands/add_dependency.rs`
- `codelet/fspec/src/add_dependency.rs`
- `codelet/fspec-core/src/help/configs/add_dependency.rs`
- `codelet/fspec-core/tests/add_dependency.rs` (13 tests pass)
- `codelet/fspec/tests/cli_add_dependency.rs` (7 tests pass)
- `codelet/fspec/tests/fixtures/help/add-dependency.txt`
- `src/commands/add-dependency.ts`
- `src/commands/add-dependency-help.ts`
- `spec/features/add-dependency-cli-subcommand.feature`
- `spec/features/add-dependency-rust-port.feature`

---

## add-tag-to-feature (RPC-193)

### Summary: FAIL (functional divergence)

### 🔴 Critical Issues

1. **Tag insertion ORDER diverges from TS when a feature has exactly one existing tag immediately above the `Feature:` line.**
   - Repro fixture (parity script `attf-new`, `attf-reminder`):
     ```
     @existing-tag
     Feature: Test feature
     ```
   - Running `fspec add-tag-to-feature spec/features/test.feature @critical`:
     - **TS** → produces `@critical\n@existing-tag\nFeature:…` (new tag goes FIRST).
     - **RUST** → produces `@existing-tag\n@critical\nFeature:…` (new tag goes AFTER existing).
   - Diff captured at parity log lines 278-286 (attf-new) and 351-359 (attf-reminder).

   **Root cause** in `codelet/fspec-core/src/commands/add_tag_to_feature.rs:307-360`:
   - TS algorithm (`src/commands/add-tag-to-feature.ts:166-189`): when the only line above Feature is a tag and `i == 0`, the inner loop hits the `if (i === 0) { insertIndex = 0; break; }` clause → `insertIndex = 0`. Then because `insertIndex (0) !== featureLineIndex (1)`, the "all blank/tags" reposition block is SKIPPED. Tag is spliced at index 0 → BEFORE the existing tag.
   - Rust port lines 327-353: the inner walk skips tag lines silently (`while i >= 0` with no `if i == 0` clamp). When `i == -1` the loop exits with `insert_at` STILL equal to `feature_line_index`. The reposition block then fires (line 343-353) and bumps `insert_at` to AFTER the last tag.
   - The Rust port is **missing the `i == 0` clamp** that the TS algorithm uses to send `insertIndex` to 0 in this corner case.

   This is a real user-visible parity break. Functionally the file still parses, message is identical, exit code is identical — but the tag ordering on disk differs, which will surface as `git diff` noise and (less commonly) as a downstream test failure for anyone who orders tags by priority.

   **Fix sketch**: in `insert_tags_before_feature`, when the backward walk reaches `i == 0` and the line is a tag, set `insert_at = 0` and `break`, matching the TS clamp.

### 🟡 Warnings

2. The TS impl writes the file **even when `valid=false`** (line 210 — unconditional `writeFile`); Rust port also does so (line 168) — parity confirmed. Note inline; not an issue.

3. `.expect("source")`-style assertions — see add-dependency note. Same pattern, same risk profile. The Rust port is generally guarded by prior validation, so these are not user-reachable today.

### Parity Matrix

| Test Case | TS Exit | TS stdout (excerpt) | Rust Exit | Rust stdout (excerpt) | Match? |
|-----------|---------|---------------------|-----------|-----------------------|--------|
| add new `@critical` to file with `@existing-tag` | 0 | `✓ Added @critical to spec/features/test.feature` + missing-required-tags reminder | 0 | identical stdout | ⛔ **file content differs (tag order)** |
| add duplicate `@existing-tag` | 1 | `Error: Tag @existing-tag already exists on this feature` | 1 | same | ✅ |
| `--validate-registry @unregistered` | 1 | `Error: Tag @unregistered is not registered in spec/tags.json` | 1 | same | ✅ |
| no flag, unregistered `@unregistered` | 0 | `✓ Added @unregistered …` + consolidated reminder (unregistered + missing-required) | 0 | identical reminder | ✅ stdout; ⛔ tag-order diff |
| `--help` (vs `node dist/index.js add-tag-to-feature --help`) | 0 | full help text | 0 | identical | ✅ byte-exact |

### Files Reviewed
- `codelet/fspec-core/src/commands/add_tag_to_feature.rs`
- `codelet/fspec/src/add_tag_to_feature.rs`
- `codelet/fspec-core/src/help/configs/add_tag_to_feature.rs`
- `codelet/fspec-core/tests/add_tag_to_feature.rs` (13 tests pass; **but** none of the existing tests cover the single-tag-above-Feature ordering corner case — coverage gap)
- `codelet/fspec/tests/cli_add_tag_to_feature.rs` (6 tests pass)
- `codelet/fspec/tests/fixtures/help/add-tag-to-feature.txt`
- `src/commands/add-tag-to-feature.ts`
- `src/commands/add-tag-to-feature-help.ts`
- `spec/features/add-tag-to-feature-cli-subcommand.feature`
- `spec/features/add-tag-to-feature-rust-port.feature`

---

## remove-tag-from-feature (RPC-281)

### Summary: PASS

### 🔴 Critical Issues
None.

### 🟡 Warnings
None substantive. Whole-line equality filter is documented as a TS quirk preserved intentionally (architecture note `[6]`). The implementation is clean, the parity test passes file byte-equality, and the help fixture matches.

### Parity Matrix

| Test Case | TS Exit | TS stdout (excerpt) | Rust Exit | Rust stdout (excerpt) | Match? |
|-----------|---------|---------------------|-----------|-----------------------|--------|
| remove `@existing-tag` from feature | 0 | `✓ Removed @existing-tag from spec/features/test.feature` | 0 | same | ✅ stdout AND file byte-equal |
| remove `@notthere` (not present) | 1 | `Error: Tag @notthere not found on this feature` | 1 | same | ✅ |
| `--help` (vs `node dist/index.js remove-tag-from-feature --help`) | 0 | full help text | 0 | identical | ✅ byte-exact |

### Files Reviewed
- `codelet/fspec-core/src/commands/remove_tag_from_feature.rs`
- `codelet/fspec/src/remove_tag_from_feature.rs`
- `codelet/fspec-core/src/help/configs/remove_tag_from_feature.rs`
- `codelet/fspec-core/tests/remove_tag_from_feature.rs` (7 tests pass)
- `codelet/fspec/tests/cli_remove_tag_from_feature.rs` (5 tests pass)
- `codelet/fspec/tests/fixtures/help/remove-tag-from-feature.txt`
- `src/commands/remove-tag-from-feature.ts`
- `src/commands/remove-tag-from-feature-help.ts`
- `spec/features/remove-tag-from-feature-cli-subcommand.feature`
- `spec/features/remove-tag-from-feature-rust-port.feature`

---

## add-tag-to-scenario (RPC-194)

### Summary: PASS (with caveat)

### 🔴 Critical Issues
None.

### 🟡 Warnings

1. **Scope drift vs feature-file architecture note `[3]`.** The Gherkin rule says scenario lookup is "top-level `Scenario:` only (NOT background, NOT outline, NOT rule-nested) — mirrors TS filter on `keyword === 'Scenario'`". The Rust impl (`add_tag_to_scenario.rs:307-324` — `find_scenario`) ALSO walks `feature.rules[*].scenarios[*]`. This is **broader** than the documented contract.

   Inspection of the TS source confirms only the top-level walk is performed (TS impl iterates `feature.children.filter(c => c.scenario && c.scenario.keyword === 'Scenario')`). The Rust port matching scenarios inside `Rule:` blocks is a behaviour DIVERGENCE — though it strictly increases parity coverage rather than reduces it. Recommend either:
   - removing the rule walk to match TS (faithful port), OR
   - updating rule `[3]` to document the broadened behaviour.

2. Same `.expect(…)` / `.unwrap_or("")` pattern as siblings; not user-visible. The `lines[scenario_line_idx]` indexing at line 231 is unguarded — if `scenario_line_idx == lines.len()` an OOB panic could fire; mitigated by prior `position(…)` find but worth a `get` + `unwrap_or`.

### Parity Matrix

| Test Case | TS Exit | TS stdout (excerpt) | Rust Exit | Rust stdout (excerpt) | Match? |
|-----------|---------|---------------------|-----------|-----------------------|--------|
| add `@smoke` to "First scenario" (has `@other`) | 0 | `✓ Added @smoke to scenario 'First scenario'` | 0 | same | ✅ |
| add `@other` (dup) to "First scenario" | 1 | `Error: Tag @other already exists on this scenario` | 1 | same | ✅ |
| add `@smoke` to "Nonexistent" | 1 | `Error: Scenario 'Nonexistent' not found in spec/features/test.feature` | 1 | same | ✅ |
| add `@smoke` to "Second scenario" (no tags) | 0 | `✓ Added @smoke to scenario 'Second scenario'` | 0 | same | ✅ |
| `--help` (vs `node dist/index.js add-tag-to-scenario --help`) | 0 | full help text | 0 | identical | ✅ byte-exact |

### Files Reviewed
- `codelet/fspec-core/src/commands/add_tag_to_scenario.rs`
- `codelet/fspec/src/add_tag_to_scenario.rs`
- `codelet/fspec-core/src/help/configs/add_tag_to_scenario.rs`
- `codelet/fspec-core/tests/add_tag_to_scenario.rs` (12 tests pass)
- `codelet/fspec/tests/cli_add_tag_to_scenario.rs` (6 tests pass)
- `codelet/fspec/tests/fixtures/help/add-tag-to-scenario.txt`
- `src/commands/add-tag-to-scenario.ts`
- `src/commands/add-tag-to-scenario-help.ts`
- `spec/features/add-tag-to-scenario-cli-subcommand.feature`
- `spec/features/add-tag-to-scenario-rust-port.feature`

---

## remove-tag-from-scenario (RPC-282)

### Summary: PASS (with caveat)

### 🔴 Critical Issues
None.

### 🟡 Warnings

1. **Same rule-walk divergence** as add-tag-to-scenario. `find_scenario` walks `feature.rules[*].scenarios[*]` though architecture note `[1]` says "top-level `Scenario:` only". `remove_tag_from_scenario.rs:215-232`. Same recommendation: either match TS strictly OR amend the spec.

2. Indexing into `lines[scenario_line_idx]` is not used in this impl (only used for boundary detection) — no OOB risk here. Clean code overall.

### Parity Matrix

| Test Case | TS Exit | TS stdout (excerpt) | Rust Exit | Rust stdout (excerpt) | Match? |
|-----------|---------|---------------------|-----------|-----------------------|--------|
| remove `@other` from "First scenario" | 0 | `✓ Removed @other from scenario 'First scenario'` | 0 | same; file byte-equal | ✅ |
| remove from "Nonexistent" (idempotent success) | 0 | `✓ Scenario 'Nonexistent' not found in spec/features/test.feature - no changes made` | 0 | same | ✅ |
| remove `@notthere` from "First scenario" (idempotent success) | 0 | `✓ No changes made - none of the specified tags found on scenario 'First scenario'` | 0 | same | ✅ |
| `--help` (vs `node dist/index.js remove-tag-from-scenario --help`) | 0 | full help text | 0 | identical | ✅ byte-exact |

### Files Reviewed
- `codelet/fspec-core/src/commands/remove_tag_from_scenario.rs`
- `codelet/fspec/src/remove_tag_from_scenario.rs`
- `codelet/fspec-core/src/help/configs/remove_tag_from_scenario.rs`
- `codelet/fspec-core/tests/remove_tag_from_scenario.rs` (8 tests pass)
- `codelet/fspec/tests/cli_remove_tag_from_scenario.rs` (6 tests pass)
- `codelet/fspec/tests/fixtures/help/remove-tag-from-scenario.txt`
- `src/commands/remove-tag-from-scenario.ts`
- `src/commands/remove-tag-from-scenario-help.ts`
- `spec/features/remove-tag-from-scenario-cli-subcommand.feature`
- `spec/features/remove-tag-from-scenario-rust-port.feature`

---

## Aggregate Test Counts (cargo)

| Suite | Pass | Fail |
|-------|------|------|
| core `add_dependency` | 13 | 0 |
| core `add_tag_to_feature` | 13 | 0 |
| core `remove_tag_from_feature` | 7 | 0 |
| core `add_tag_to_scenario` | 12 | 0 |
| core `remove_tag_from_scenario` | 8 | 0 |
| cli `cli_add_dependency` | 7 | 0 |
| cli `cli_add_tag_to_feature` | 6 | 0 |
| cli `cli_remove_tag_from_feature` | 5 | 0 |
| cli `cli_add_tag_to_scenario` | 6 | 0 |
| cli `cli_remove_tag_from_scenario` | 6 | 0 |
| **TOTAL** | **83** | **0** |

All existing cargo tests are green AND all five `--help` fixtures match TS byte-for-byte — but **the test suite as-written does not catch two real parity bugs** because no test exercises:
1. The single-tag-above-Feature ordering corner case for `add-tag-to-feature`.
2. The JSON key-order preservation for `add-dependency` (and likely all work-units.json mutators).

---

## Parity Run Artifacts

- Full parity log: `/tmp/parity-batch4-1781162580.txt`
- Help diagnostics: `/tmp/help-diag-1781162731.txt`
- Per-test cargo outputs: `/tmp/test-add_dependency.txt`, `/tmp/test-add_tag_to_feature.txt`, `/tmp/test-remove_tag_from_feature.txt`, `/tmp/test-add_tag_to_scenario.txt`, `/tmp/test-remove_tag_from_scenario.txt`, plus `/tmp/test-cli-*` counterparts.

---

## Recommended Supervisor Actions (Priority Order)

1. 🔴 **FIX** `add-tag-to-feature` insertion algorithm: add the `i == 0 → insert_at = 0` clamp at `codelet/fspec-core/src/commands/add_tag_to_feature.rs:330-337` mirroring TS lines 174-177. Add a regression test for the `@existing-tag\nFeature:` → `@new\n@existing-tag\nFeature:` case.
2. 🟡 **DECIDE** on `add-dependency` JSON key-order: either restructure `WorkUnitsData` so `prefixes/epics/states` appear in TS order, OR accept the noise and document the divergence on RPC-177's rules. This decision will affect every work-units.json-mutating command in the port.
3. 🟡 **DECIDE** on the rule-nested scenario walk in `add-tag-to-scenario` / `remove-tag-from-scenario`: shrink to top-level only (faithful), OR amend Gherkin rules to declare the broadened behaviour.
4. 🟢 Optional: replace `.expect(…)` with `.ok_or(FspecCoreError::Internal{…})?` in the four commands that share this pattern, for stricter "no panics in production code" hygiene.
