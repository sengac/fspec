# Review: RPC-068 — Final TS-frontend regression + boundary audit

**Date:** 2026-05-26
**Reviewer:** Claude Code (fspec review skill)
**Mode:** Single-card review (no children — leaf story under RPC-030)

## Status: WARN

## 🔴 Critical Issues (Must Fix)
None.

## 🟡 Warnings (Should Fix)
1. **Test file exceeds 300-line guideline.** `src/__tests__/rpc-068-boundary-audit.test.ts` is 366 lines.
   CLAUDE.md mandates: *"Keep files under 300 lines — refactor when approaching this limit."*
   Worse, the helper file `src/__tests__/helpers/rpc-068-audit-helpers.ts` opens with a comment
   claiming the helpers were extracted *"so the main test file stays under the 300-line guideline
   in CLAUDE.md"* — which is no longer true. Either trim the test or extract more code into the
   helper module.

## 🟢 Observations (Nice to Have)
1. **Documented test-count numbers are internally inconsistent.**
   - Feature scenario 5 last step says: *"a full `npm test` run reports 4747 passing tests… with
     the 27 remaining failures all in pre-existing Ink-rendering test files"*.
   - The audit report's own Section 4.2 enumerates only **16** Ink-rendering failures
     (2 + 5 + 2 + 4 + 2 + 1 across six files).
   - Section 4.1 of the audit says watch-024 went from 11/16 failing to 16/16 passing after
     this card's fix. That implies the post-fix state is **4758 pass + 16 fail**, not
     **4747 pass + 27 fail**.
   - The discrepancy is documentation-only: the test file does **not** enforce the 4747 / 27
     numbers, so no test breaks. Still, future agents re-reading the spec/report could be
     confused. This is **out of scope** for a defect fix on a `done` card — the numbers are
     already committed in approved acceptance criteria — but flagged here for transparency.

## Coverage Verification
- Feature file: `spec/features/rpc-068-final-ts-regression-and-boundary-audit.feature` — **OK**
  - All 6 scenarios have Given/When/Then ordering correct.
  - Architecture doc string present and accurate.
  - `@RPC-068` tag present (matches uppercase convention used across the entire RPC-030 chain).
  - Other tags (`@done`, `@coverage-tracking`, `@source-shape`, `@regression`, `@testing`, `@rpc`)
    are all registered in the tag registry.
  - `fspec validate` passes.
- Test file: `src/__tests__/rpc-068-boundary-audit.test.ts` — **OK** (with size warning above)
  - Every scenario has a matching `it(...)` block.
  - Every Gherkin step has a matching `// @step ...` comment with text matching verbatim.
  - All 6 tests pass under vitest.
- Helper file: `src/__tests__/helpers/rpc-068-audit-helpers.ts` — **OK**
  - 86 lines, well under the size limit.
  - Pure helpers, no `any`, no `as unknown as`, ES-modules style, proper JSDoc.
- Impl files (artefacts that the test asserts against):
  - `codelet/sessions/src/session_manager.rs` — exists, contains `broadcast::Sender<(SessionId, StreamChunk)>` (verified).
  - `codelet/sessions/src/background_session.rs` — exists (verified).
  - `codelet/napi/src/persistence/` contains exactly `mod.rs` + `napi_bindings.rs` (verified).
  - `codelet/napi/src/session_manager.rs` — deleted (verified).
  - `codelet/napi/index.d.ts` — 196 declared functions, baseline 191, +5 additive (verified).
- Boundary audit report: `spec/attachments/RPC-068/boundary-audit-report.md` — **OK**
  - 354 lines, covers every verification-matrix row.
  - Includes pass/fail counts, the index.d.ts diff, dependency-rule results.
  - Contains the literal phrase "RPC-030 is hereby considered complete" (test enforces this).
- Scenario coverage: **6 / 6 scenarios covered (100%)**.

## Quality-of-Code Check
| Check | Result |
|---|---|
| No `any` types in test / helper | ✅ |
| No `as unknown as` casts | ✅ |
| ES modules only (no `require`) | ✅ |
| No `var`, no `==`/`!=` | ✅ |
| No `console.log` in production code | ✅ |
| Curly braces on all `if`/`else` | ✅ |
| `interface` (not `type`) for object shapes | N/A (no object shapes defined in this slice) |
| `import type` for type-only imports | N/A |
| All promises awaited or voided | ✅ |
| No floating dynamic imports | ✅ |
| File extensions absent from TS imports | ✅ |
| **File ≤ 300 lines** | ❌ test file is **366** lines |

## Build & Test Verification
- `npx vitest run src/__tests__/rpc-068-boundary-audit.test.ts` → **6 / 6 pass**
- `npx vitest run src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts` → **16 / 16 pass**
- `fspec validate` on the feature file → ✅
- `fspec audit-coverage rpc-068-final-ts-regression-and-boundary-audit` → ✅ (12/12 mappings valid)

## Files Reviewed
- `spec/features/rpc-068-final-ts-regression-and-boundary-audit.feature`
- `spec/attachments/RPC-068/final-regression-and-audit.md`
- `spec/attachments/RPC-068/boundary-audit-report.md`
- `spec/attachments/RPC-068/ast-research-boundary-audit.md`
- `src/__tests__/rpc-068-boundary-audit.test.ts`
- `src/__tests__/helpers/rpc-068-audit-helpers.ts`
- `src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts` (first 100 lines, structural check)
- `codelet/napi/src/persistence/` (directory listing)
- `codelet/sessions/src/` (directory listing)
- `codelet/napi/src/session_manager.rs` (confirmed absent)

---

## Fix Plan
- Move the work unit back to `implementing` so the fix is captured under the same ACDD trace.
- Trim verbose inline rationale comments and consolidate repetitive boilerplate in
  `rpc-068-boundary-audit.test.ts` until it sits under 300 lines. Preserve every `// @step`
  comment verbatim and every assertion.
- Re-run the test to confirm no regression.
- Re-run `fspec audit-coverage` and `fspec validate`.
- Advance back through `validating` → `done`.

---

## Fix Results

### RPC-068: Final TS-frontend regression + boundary audit
- 🟡 **Test file size 366 → 255 lines** → ✅ Fixed.
  - Extracted reusable constants into `src/__tests__/helpers/rpc-068-audit-helpers.ts`:
    - `codeRootDirs(codelet)` — returns the nine `*/src` directories the
      GLOBAL_CHUNK_CALLBACK scan traverses.
    - `LIFTED_PERSISTENCE_MODULES` — the six `.rs` modules that RPC-031..RPC-035
      lifted into `codelet/core/src/persistence/`.
    - `ADDITIVE_NAPI_EXPORTS` — the five additive `export declare function`
      identifiers (countCheckpoints, getModelInfo, getWorkspaceInfo,
      moveWorkUnitUp, moveWorkUnitDown).
    - `WATCH_024_REQUIRED_SOURCE_FILES` — the seven post-RPC-030 source files
      the watch-024 supervisor-terminology test now reads.
    - `VERIFICATION_MATRIX_ROWS` — the fifteen verification-matrix row labels
      the audit report must contain.
  - Flattened multi-line `existsSync(join(CODELET, ..., ..., ...))` chains into
    single-line calls, since the path segments were the bulkiest part of the file.
  - Replaced verbose rationale comment paragraphs with concise one-line
    explanations; every `// @step` comment is preserved verbatim.
  - Helper file grew from 86 → 169 lines (still well under 300).
  - The leading comment in the helper file (`"Kept in a separate module so the
    main test file stays under the 300-line guideline in CLAUDE.md"`) is now
    accurate again.
- 🟢 **Documented test-count discrepancy (27 vs 16 Ink failures)** → Not fixed.
  - The numbers live in already-committed acceptance criteria and in the audit
    report. The test file does not enforce them, so no test regression results
    from the inconsistency. Changing them would be a spec change, not a defect
    fix — left untouched per the user's "no scope creep" directive. Flagged
    here so a future bug card can correct the documentation if desired.

## Final Verification
- All RPC-068 tests pass: ✅ (`npx vitest run src/__tests__/rpc-068-boundary-audit.test.ts` — 6 / 6 pass).
- Watch-024 still passes: ✅ (`npx vitest run src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts` — 16 / 16 pass).
- Build succeeds: ✅ (`npm run build` — full release rebuild including the NAPI Rust workspace, then vite bundle).
- Coverage complete: ✅ (`fspec audit-coverage rpc-068-final-ts-regression-and-boundary-audit` — 12 / 12 mappings valid).
- Feature file valid: ✅ (`fspec validate spec/features/rpc-068-final-ts-regression-and-boundary-audit.feature`).
- Test file ≤ 300 lines: ✅ (255 lines).
- Helper file ≤ 300 lines: ✅ (169 lines).
