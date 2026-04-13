# SessionHeader Critical Review — Investigation Report

**Date:** 2026-04-12
**Investigator:** Claude Code (3 parallel agents)
**Source Report:** ~/Desktop/REPORT.md
**Review Criteria:** spec/skills/review-skill.md

---

## Investigation Method

3 subordinate agents ran in parallel, each investigating a subset of the 10 critical issues using DeepSearch, AstGrep, Grep, and Read tools against the actual codebase:

| Agent | Focus | Issues |
|-------|-------|--------|
| Agent 1 — Store & State | `sessionStore.ts` | C1, C2, C6 |
| Agent 2 — Component & Rendering | `SessionHeader.tsx`, utils, views | C3, C4, C5, C9 |
| Agent 3 — Test Quality | 3 test files | C7, C8, C10 |

---

## Consolidated Verdicts

| Issue | Verdict | Severity | Child Card |
|-------|---------|----------|------------|
| **C1** — sessionStore 415 lines | ✅ CONFIRMED | Medium | TUI-086 |
| **C2** — DRY violation (4 functions) | ✅ CONFIRMED | **High** | TUI-086 |
| **C3** — supervisorInfo dead code | ✅ CONFIRMED | Low-Med | TUI-087 |
| **C4** — modelId ANSI injection | ❌ REFUTED | Negligible | None |
| **C5** — formatContextWindow inconsistency | ✅ CONFIRMED | Low | TUI-088 |
| **C6** — setIsolationState desync | ✅ CONFIRMED | Medium | TUI-086 |
| **C7** — SessionHeader.test 419 lines | ✅ CONFIRMED | **High** | TEST-036 |
| **C8** — Thinking level test 567 lines | ✅ CONFIRMED | Med-High | TEST-037 |
| **C9** — formatPercentage in render | ✅ CONFIRMED | Negligible | TUI-088 |
| **C10** — E2E "NO MOCKS" claim | ⚠️ PARTIAL | Low-Med | TEST-038 |

**9 of 10 confirmed. 1 refuted (C4).**

---

## C4 Refutation Details

The report claimed `modelId` from Rust NAPI is unsanitized and vulnerable to ANSI escape injection. Investigation traced the full data flow:

1. `modelId` arrives as a prop from `AgentView.tsx` line 5145: `modelId={displayModelId}`
2. `displayModelId` comes from a `useMemo` at line 1212, resolving through:
   - **Rust NAPI path:** `rustModel.modelId` → `session_manager.rs` `self.model_id` (RwLock<Option<String>>) set only by `session_set_model()` or `session_set_model_profile()`
   - **Local state path:** `currentModel?.displayName || currentModel?.modelId || currentProvider` from provider model lists
3. Both sources are **application-controlled data** — no external/user-input pathway exists
4. The claim incorrectly linked `modelId` to `sessionGetStatus` (which returns session status strings, not model IDs)

**Conclusion:** No action needed. Theoretical concern with no realistic attack vector.

---

## Priority Order for Fixes

1. **TUI-086** — sessionStore refactoring (C1 + C2 + C6) — highest impact, most code debt
2. **TEST-036** — SessionHeader test overhaul (C7) — 58% of tests don't test runtime behavior
3. **TEST-037** — Thinking level test split (C8) — 89% over line limit
4. **TUI-087** — Dead code removal (C3) — straightforward cleanup
5. **TUI-088** — Format utilities (C5 + C9) — cosmetic consistency
6. **TEST-038** — Test description fix (C10) — documentation accuracy
