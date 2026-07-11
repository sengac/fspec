# Review: BUG-150 & BUG-151 — ACDD Compliance Review

**Date:** 2026-07-11
**Reviewer:** Claude Code (fspec review skill, 2 parallel review agents)
**Work Units Reviewed:** 2 (BUG-150, BUG-151)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 8 (6 on BUG-151, 2 on BUG-150 — one report-only cross-cutting)
- 🟢 Observations: 8

---

## BUG-151: add-attachment self-copy truncation — WARN

All tests pass (18 TS / 15 Rust), clippy clean, fix correctly ordered in both
implementations, coverage 6/6 + 15/15. No behavioral defects in the fix itself.

### 🟡 Warnings
1. **Stale architecture doc string** in `spec/features/add-attachment-rust-port.feature`
   (lines 12–15): claims Mermaid validation is OMITTED from the Rust port, but the feature
   has two Mermaid scenarios (lines 100–110) and `add_attachment.rs:130–142` validates via
   the real merman parser. Doc string actively misleads.
2. **Blanket implLines (1–250) for the 11 pre-BUG-151 scenarios** in
   `add-attachment-rust-port.feature.coverage`: includes 67 rustdoc lines, excludes
   251–289 (meta bump, atomic write, output rendering that the Description scenarios
   assert). Pre-existing from RPC-170, surfaced by this review. The 4 new BUG-151
   mappings are precise.
3. **Rust parity gap — 4 of 6 BUG-151 scenarios ported**: Rust feature/tests omit
   "Register a read-only file already in the attachments directory without attempting a
   copy" and "Duplicate registration from a different source file does not overwrite the
   registered attachment". Both meaningful in Rust.
4. **Asymmetric canonicalization fallback** in `add_attachment.rs:200–205`: each side
   falls back to lexical resolution independently; TS (`add-attachment.ts:87–93`)
   canonicalizes both-or-neither. Mixed-form comparison could yield a false
   "not self-copy" → the exact truncation this bug fixed. Use both-or-neither in Rust.
5. **Rust BUG-055 unlink swallows errors** (`add_attachment.rs:229`,
   `let _ = remove_file(...)`) while TS propagates (`add-attachment.ts:109`). Divergent
   failure behavior — align to propagate.
6. **Wrong feature reference in Rust test section header**
   (`codelet/fspec-core/tests/add_attachment.rs:557`): says
   `work-unit-attachments.feature` but the 4 tests map to
   `add-attachment-rust-port.feature` scenarios (lines 112–140).

### 🟢 Observations
1. Stale "Red phase / NotYetPorted stub" headers in
   `codelet/fspec-core/tests/add_attachment.rs:7–10` and
   `codelet/fspec/tests/cli_add_attachment.rs:10`.
2. `work-unit-attachments.feature` Feature: line is the raw bug-symptom title rather than
   the capability name.
3. Verbatim scenario duplication across the two feature files (deliberate repo-wide
   TS/Rust two-front pattern) can drift — Rust copy already dropped 2 scenarios (W3).
4. TOCTOU window between canonicalize/compare and copy (both impls). Accepted risk for a
   CLI tool; noted for the record.

---

## BUG-150: bug146 time-bound git-diff assertion — PASS

Feature/tests/coverage verified 9/9; clippy clean with --all-targets; zero git
invocations remain; sibling scenario reads on-disk source only; BUG-152 corruption
recovery verified clean on this file.

### 🟡 Warnings
1. **Same time-bound git-diff disease elsewhere (report-only — needs follow-up card):**
   - `codelet/napi/tests/session_bindings_shape.rs:976–985` — asserts on the working-tree
     `git diff codelet/napi/index.d.ts` shape (RPC-043 era).
   - `codelet/fspec-tui/tests/rpc027_dialog_parity_ij.rs:168–176` — asserts "git diff
     against the base branch is empty"; depends on branch/tree state.
   - Softer instances (phrasing only, audit when touched):
     `codelet/sessions/tests/session_manager_shape.rs:86,1587,1653`,
     `codelet/sessions/tests/background_session_shape.rs:1079`,
     `codelet/sessions/tests/rpc076_session_manager_handle_imports_shape.rs:15`.
2. **Pre-existing scenario-fidelity gap in scenario 6** ("JSON round-trip tests still
   pass with the napi feature", test lines 317–363): Gherkin says
   `cargo test -p codelet-rpc-types --features napi` but the test runs
   `cargo check --tests` (documented PRAGMATIC NOTE — napi linking impossible host-side).
   Pre-existing from BUG-146; reword the Gherkin step to match reality.

### 🟢 Observations
1. Multiplicity pin scans raw source including comments (test lines 418–427, 434–439);
   zero-`napi(js_name` check filters comment lines but the serde(rename counts do not.
   Negligible risk today; filter `code_lines` there too for tightness.
2. Architecture notes are a `#` comment block (feature lines 9–43) rather than a doc
   string; content excellent and survived BUG-152 intact.
3. Impl mapping is a whole-file pin (lib.rs 1–1858) for all 9 scenarios — coarse but
   defensible: the pinned artifact is the on-disk source shape.
4. Rustdoc rationale for the BUG-150 rewrite present and accurate (test lines 369–377,
   header 15–16).

---

## Fix Plan
- BUG-151: fix W1–W6 + O1 (cheap doc hygiene). O2–O4 deferred (recorded here).
- BUG-150: fix W2 (reword scenario 6 step + @step sync + re-link) and O1 (filter comment
  lines in the multiplicity scan); W1 handled by filing a follow-up bug card for the two
  hard instances. O2/O3 accepted as-is.

---

## Fix Results (2026-07-11, post-review fix pass)

### BUG-151 — all fixed, re-validated to done
- 🟡 W1 stale Mermaid claim in architecture doc string → ✅ Fixed via add-architecture (doc string now documents merman validation + BUG-151 ordering)
- 🟡 W2 blanket implLines on 11 legacy scenarios → ✅ All 17 scenarios re-linked with precise ranges against the current 335-line impl (run() = 97–305; register-only scenarios exclude the copy/unlink block)
- 🟡 W3 Rust parity gap → ✅ 2 scenarios ported (read-only register-only; different-source duplicate) + 2 tests (lines 741–861) with sabotage red-proof (guards disabled → EACCES / silent-success failures observed; impl restored hash-verified)
- 🟡 W4 asymmetric canonicalization → ✅ Both-or-neither match on canonicalize results (add_attachment.rs:206–213), TS parity
- 🟡 W5 swallowed BUG-055 unlink error → ✅ Now propagates as FspecCoreError (add_attachment.rs:236–246)
- 🟡 W6 wrong feature reference in test section header → ✅ Corrected to add-attachment-rust-port.feature
- 🟢 O1 stale "NotYetPorted stub" headers → ✅ Removed in both test files

### BUG-150 — fixed, re-validated to done
- 🟡 W2 scenario 6 fidelity (cargo test vs cargo check) → ✅ Gherkin reworded to "type-check the test suite with the napi feature enabled"; @step comments synced; coverage re-linked
- 🟡 W1 cross-cutting git-diff disease → ✅ Follow-up card BUG-153 filed (backlog, relatesTo BUG-150): session_bindings_shape.rs:976–985 + rpc027_dialog_parity_ij.rs:168–176, softer instances listed for audit-when-touched
- 🟢 O1 multiplicity pin scans comments → ✅ Counts now over comment-filtered code_lines; re-proved regression sensitivity (variant-B red-proof re-run, revert hash-verified)

### Final Verification (independently re-run by supervisor)
- cargo test -p codelet-fspec-core --test add_attachment: 17/17 ✅
- cargo test -p codelet-rpc-types (all binaries): 38 passed, 0 failed ✅
- cargo clippy -p codelet-fspec-core --all-targets: clean ✅
- cargo clippy -p codelet-rpc-types --all-targets: clean ✅
- cargo test -p codelet-fspec --test cli_add_attachment: 10/10 ✅
- Vitest attachment suites: 18/18 ✅
- Coverage: work-unit-attachments 6/6, add-attachment-rust-port 17/17, rpc-types feature 9/9 ✅
- BUG-150 done, BUG-151 done, BUG-153 backlog ✅
