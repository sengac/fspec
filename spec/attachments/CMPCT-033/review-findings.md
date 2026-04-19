# Review: CMPCT-033 — Compaction SessionStateChange drops sessionId — wrong session shows Compacting indicator

**Date:** 2026-04-18
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (no children — standalone bug)
**Type:** bug · **Estimate:** 3 · **Current Status:** done

## Status: ✅ PASS

---

## 🔴 Critical Issues (Must Fix)

**None.**

All four scenarios are fully covered, all @step comments match the Gherkin text, tests run green, the build succeeds, `fspec validate` and `fspec audit-coverage` both pass, and the fix correctly follows the routed-sessionId outer-routing pattern called out in the architecture notes.

---

## 🟡 Warnings (Should Fix)

1. **Guard consistency across the three compaction consumers.**
   - `persistentSessionStateHandler.ts:67` — ✅ guards with `if (sessionId) { … }` before calling `startCompaction`.
   - `AgentView.tsx:3408` — ✅ guards with `if (routedSessionId) { … }` before calling `startCompaction`.
   - `AgentView.tsx:2354-2366` (inside the `handleSubmit` inner `attachToSession` callback) — ❌ has **no guard**. If `routedSessionId` is ever the empty string, `sessionGetCompactionProgress('')` and `compactionRef.current.startCompaction('hook-triggered', '', …)` will still be called.

   In practice this path only fires for a session that has already been attached via `attachToSession(activeSessionId, …)`, so `routedSessionId` should always be non-empty — but the other two sites guard defensively, and the regression test `does not start compaction when routed sessionId is an empty string` explicitly encodes that contract for the handler. The inline branch in `AgentView.tsx:2354-2366` should match, either by adding `if (routedSessionId) { … }` or by routing through `handlePersistentSessionStateChange` so there is a single enforcement point.

2. **`refreshRustState(activeSessionId)` at `AgentView.tsx:2372` intentionally uses the viewed session, but the inline comment block (2346-2371) never explains *why* it diverges from the `routedSessionId` used two lines above.** The extracted handler (`persistentSessionStateHandler.ts:80-84`) has a detailed block comment explaining this split; the inline SessionStateChange branch inside `handleSubmit` does not, which is a future-maintenance trap (someone will "fix" it to `routedSessionId` and regress BUG-101). Copy the explanatory comment from the handler.

---

## 🟢 Observations (Nice to Have)

1. **`AgentView.tsx` is 5,661 lines.** Pre-existing, not introduced by CMPCT-033 — but this card adds a third `SessionStateChange`/`Compacting` branch to the file (alongside the extracted handler). The extraction pattern proven here (`persistentSessionStateHandler`) is exactly how the other two inline branches should eventually be lifted out. Consider a follow-up refactor ticket to route `handleSubmit`'s inner handler and `handleStreamChunk` through the same extracted function so CMPCT-033's one-site-of-truth design actually has one site of truth.

2. **Test file naming vs. feature file naming.** The test file is `persistentSessionStateHandler-routed-sessionid.test.ts` (focused on the extracted handler), which is fine. But the feature file covers all three consumers per rule `[3]`, and only the handler has direct unit tests. The two inline branches in `AgentView.tsx` are covered *by line-range linking only*, not by assertions. That's acceptable for a TUI component where unit-level isolation is hard, but it means rules `[3]` and `[4]` are verified structurally (the line ranges show the `routedSessionId` is used) rather than behaviourally. Worth noting for the reviewer's mental model — the coverage audit is green because file ranges match, not because those two branches have dedicated tests.

3. **`persistentSessionStateHandler.ts` docstring on the `sessionId` parameter (line 51)** says *"The routed sessionId the chunk arrived for (NOT the currently-viewed session)"*. Excellent — this is exactly the kind of prose that prevents the regression from reappearing. Consider mirroring the same wording inline at the `AgentView.tsx:3175` comment for `handleStreamChunk`'s `routedSessionId` parameter (the current comment says "Accepts the routed sessionId so SessionStateChange(Compacting) attaches to the session that emitted the chunk rather than the currently-viewed session" — which is good but slightly less explicit about *not* using `currentSessionIdRef.current`).

---

## Coverage Verification

- **Feature file:** `spec/features/compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator.feature` — ✅ OK (valid Gherkin, 4 scenarios, architecture doc string present, all required tags including `@CMPCT-033`, `@bug-fix`, `@compaction`, `@session-management`, `@tui`, `@done`).
- **Test file:** `src/tui/handlers/__tests__/persistentSessionStateHandler-routed-sessionid.test.ts` — ✅ OK (feature-file header present, every scenario has an `it()` block, @step comments match Gherkin step text verbatim, plus three regression tests for Cleared/Idle/empty-sessionId).
- **Impl files:**
  - `src/tui/handlers/persistentSessionStateHandler.ts` — ✅ OK.
  - `src/tui/services/globalSessionStreamManager.ts:23-26` — ✅ OK (`SessionChunkHandler` signature now `(sessionId, chunk) => void`, matching the architecture note).
  - `src/tui/components/AgentView.tsx` — ✅ OK at lines 988-1020, 1981, 2354-2366, 3175-3176, 3408-3415, 3671-3672, 4309-4310 (all three consumers receive and use `routedSessionId`). ⚠️ Minor: missing guard at 2354-2366 (see Warning 1).
- **Scenario coverage:** 4/4 scenarios covered (100%). `fspec audit-coverage` reports all 10 file references valid.

---

## Verification Commands Run

| Command | Result |
|---|---|
| `npm test -- src/tui/handlers/__tests__/persistentSessionStateHandler-routed-sessionid.test.ts` | ✅ 7/7 pass (406 ms) |
| `npm run build` (via npm test) | ✅ `dist/index.js  2,103.63 kB` |
| `fspec validate spec/features/compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator.feature` | ✅ valid |
| `fspec audit-coverage compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator` | ✅ 10/10 files found, all mappings valid |
| `fspec show-coverage …` | ✅ 100% (4/4 scenarios) |

---

## Files Reviewed

- `spec/features/compaction-sessionstatechange-drops-sessionid-wrong-session-shows-compacting-indicator.feature`
- `src/tui/handlers/persistentSessionStateHandler.ts`
- `src/tui/handlers/__tests__/persistentSessionStateHandler-routed-sessionid.test.ts`
- `src/tui/services/globalSessionStreamManager.ts`
- `src/tui/hooks/useSessionStreamManager.ts`
- `src/tui/components/AgentView.tsx` (relevant blocks: 970-1020, 1970-2000, 2340-2380, 3165-3180, 3385-3425, 3670-3675, 4305-4315)
- Work-unit data via `fspec show-work-unit CMPCT-033`
- Coverage data via `fspec show-coverage …` and `fspec audit-coverage …`

---

## Summary

- 🔴 Critical: **0**
- 🟡 Warnings: **2** (guard-consistency asymmetry at AgentView.tsx:2354-2366; missing divergence comment for refreshRustState)
- 🟢 Observations: **3** (file size, inline-vs-extracted coverage, docstring mirroring)

CMPCT-033 is correctly done. The fix faithfully implements all four architecture notes — the NAPI ABI was untouched, the `SessionChunkHandler` type signature was widened, and all three compaction consumers pass the routed sessionId through to `startCompaction` / `getCompactionProgress`. The regression-risk remaining is cosmetic (one un-guarded branch, one missing comment) and does not change behaviour for any traced code path.

**Recommendation:** keep `done` status. File a small follow-up nit to address Warning 1 + 2 if the inline `SessionStateChange` branches in `AgentView.tsx` are ever touched again, or (better) extract both remaining inline branches into `handlePersistentSessionStateChange` so the guard and comment live in exactly one place.
