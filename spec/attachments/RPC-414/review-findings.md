# Review: RPC-414 — Fspec tool help unreachable in TUI

**Date:** 2026-07-07
**Reviewer:** Claude Code (fspec review skill) — independent review worker `bd3d6537`
**Work Units Reviewed:** 1 (bug; no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 0 (none blocking)
- 🟢 Observations: 4 (2 actioned, 2 confirmations of correct behavior)

## Review Result: RPC-414 — PASS

### 🔴 Critical Issues
None.

### 🟡 Warnings
None blocking.

### 🟢 Observations (and disposition)
1. **`help --help` precedence undocumented** (`help_dispatch.rs` try_dispatch_help)
   → ✅ **Fixed**: added a precedence comment documenting that the literal `help`
   command (Shape 1/2) is matched before the trailing-flag form (Shape 3), so
   `"help --help"` resolves to general help.
2. **`strip_trailing_help_flag` correctness** — reviewer confirmed correct handling of
   bare `"  --help"` (→ None → normal dispatch) and rejection of names ending in `-h`
   without preceding whitespace via the `prefix.ends_with(char::is_whitespace)` guard.
   → No fix needed (confirmation of correct behavior).
3. **`GENERAL_HELP` text is factual** — describes only the two real discovery mechanisms,
   no fabricated command behavior (rule [4] compliant).
   → No fix needed (confirmation).
4. **Table/module drift risk** — `help_dispatch_table.rs` is a hand-maintained mirror of
   `help/configs/mod.rs`; a future `pub mod` without a table entry would silently degrade.
   → ✅ **Fixed**: added a drift-guard test module in `help_dispatch_table.rs` that reads
   `help/configs/mod.rs` at test time and asserts `config_for` is 1:1 with the registered
   modules, plus a test asserting the 5 known no-CONFIG commands degrade gracefully
   (config_for → None, but canonical lookup → Some).

## Coverage Verification
- Feature file: `spec/features/fspec-tool-help-dispatch.feature` — OK (7 scenarios,
  `@RPC-414` + required component/feature-group tags, architecture doc string, `@done`,
  no placeholders, correct Given/When/Then ordering)
- Test file: `codelet/fspec-core/tests/rpc414_help_dispatch.rs` — OK (7 tests 1:1 with
  scenarios; header references feature file; all 29 `@step` comments byte-for-byte match)
- Impl files:
  - `codelet/fspec-core/src/help_dispatch.rs` (132 LoC) — OK
  - `codelet/fspec-core/src/help_dispatch_table.rs` (282 LoC incl. drift-guard tests) — OK
  - `codelet/fspec-core/src/dispatch.rs` — help pre-step wired at :107-110 BEFORE canonical
    `lookup` at :118 — OK
  - `codelet/fspec-core/src/lib.rs:30-31` — modules registered — OK
- Scenario coverage: **7/7 (100%)**; coverage audit: 14/14 files found, all mappings valid.

## Build/Test/Clippy Results (post-fix, verified by supervisor)
- `cargo build -p codelet-fspec-core` — clean, no warnings.
- `cargo test -p codelet-fspec-core --test rpc414_help_dispatch` — **7 passed / 0 failed**.
- New parity tests (`help_dispatch_table::tests`) — **2 passed / 0 failed**.
- `cargo test -p codelet-fspec-core` (whole crate) — **2133 passed / 0 failed / 3 ignored**
  (3 ignored are pre-existing NAPI-delegation stubs; +2 vs the pre-review 2131 = the new
  drift-guard tests).
- `cargo clippy -p codelet-fspec-core` — no warnings on the new files.

## Fix Results
### RPC-414: Fspec tool help unreachable in TUI
- 🟢 Observation 1 (precedence doc) → ✅ Fixed: comment added in `try_dispatch_help`.
- 🟢 Observation 4 (table drift risk) → ✅ Fixed: self-maintaining parity test + no-CONFIG
  graceful-degradation test added to `help_dispatch_table.rs`.
- 🟢 Observations 2 & 3 → confirmed correct; no change required.

## Final Verification
- All tests pass: ✅ (2133 / 0 / 3 ignored)
- Build succeeds: ✅
- Clippy clean: ✅
- Coverage complete: ✅ (7/7, audit valid)
- Feature file valid: ✅
- Tags valid: ✅
