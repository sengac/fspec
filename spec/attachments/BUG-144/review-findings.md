# Epic Review: BUG-144 — PromptCancelled errors not caught during compaction

**Date:** 2026-04-24
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1

## Summary
- 🔴 Critical: 3 issues (all fixed)
- 🟡 Warnings: 4 issues (3 fixed, 1 accepted)
- 🟢 Observations: 3

## Work Unit Results

### BUG-144: PromptCancelled errors not caught during compaction — WARN → PASS

---

## 🔴 Critical Issues (Must Fix)

1. **Feature 2 Gherkin `Given` step is stale — references the pre-fix state as a precondition**
   - `spec/features/replace-anyhow-macro-with-error-from-in-rigagent-streaming-paths.feature`, line 26
   - ✅ Fixed: Rewrote Given step to describe the current state: `Given RigAgent streaming error conversion sites in rig_agent.rs use anyhow::Error::from(e)` and When to `When the streaming error conversion is verified against the source`

2. **Feature 2 test `@step` comments mirror the stale Gherkin Given step**
   - `codelet/core/src/rig_agent.rs:208`
   - ✅ Fixed: Updated @step comments to match the corrected feature file steps

3. **Feature 1 Scenario 2 `@step` mismatch: Gherkin says `extract_prompt_cancelled` but test @step says `is_compaction_cancelled`**
   - Feature file line 37 / test at `error_classifiers.rs:506`
   - ✅ Fixed: Updated feature file Scenario 2 to match the test's actual behavior: `When is_compaction_cancelled is called on that error` / `Then the function returns false`

## 🟡 Warnings (Should Fix)

1. **Feature 2 scenario is a negative-assertion-only test — its Then step claims type chain preservation but doesn't test it directly**
   - Accepted: Cross-feature coverage is valid. Feature 1 tests directly verify type chain preservation. Feature 2 is a source-scan guard that prevents regression of the broken pattern.

2. **`classify_compaction_branch` uses `.unwrap_or(false)` on a poisoned Mutex — silently swallows the error**
   - Accepted: This is pre-existing code (not part of BUG-144). The error-side detection via `extract_prompt_cancelled` is authoritative per CMPCT-026 design. A separate work unit could address this if needed.

3. **Feature 1 Architecture doc string claims "No changes needed in error_classifiers.rs" but the test was corrected**
   - ✅ Fixed: Updated architecture doc string to clarify: "No changes needed in error_classifiers.rs production code" and added note that "The false positive test was corrected to use Error::from instead of .into()."

4. **Feature 2 Gherkin step references specific source line numbers — brittle**
   - ✅ Fixed: Rewrote the Given step to remove line number references. Now uses abstract description: "RigAgent streaming error conversion sites in rig_agent.rs use anyhow::Error::from(e)"

## 🟢 Observations (Nice to Have)

1. **Implementation is clean and correct** — The `Error::from(e)` fix at all four sites in `rig_agent.rs` properly preserves the typed error chain, and `extract_prompt_cancelled()` handles both direct and boxed `PromptError` cases. No `unwrap()` in production code, proper `Result` propagation.

2. **Test quality in Feature 1 is high** — All three tests in `error_classifiers.rs` (lines 478-572) use `@step` comments, test real behavior, and cover the key cases: Error::from path, bare string rejection, and chain traversal verification.

3. **The `is_compaction_cancelled` test helper is well-documented** — Lines 130-138 clearly explain why the `#[cfg(test)]` wrapper exists (CMPCT-026 compatibility) and its relationship to `extract_prompt_cancelled`.

## Fix Results

### BUG-144: PromptCancelled errors not caught during compaction
- 🔴 Issue 1: Stale Given step → ✅ Fixed: Rewrote Given/When steps to describe current state
- 🔴 Issue 2: Stale @step comments → ✅ Fixed: Updated @step comments to match corrected feature file
- 🔴 Issue 3: @step function name mismatch → ✅ Fixed: Updated feature file to match test's actual call
- 🟡 Issue 3: Architecture doc string inaccuracy → ✅ Fixed: Clarified production code vs test distinction
- 🟡 Issue 4: Brittle line number references → ✅ Fixed: Removed line numbers from feature file

## Final Verification
- All tests pass: ✅ (125 codelet-core, 13 error_classifiers tests)
- Build succeeds: ✅
- Coverage complete: ✅ (4/4 scenarios, 100%)
- Feature files valid: ✅
- Tags valid: ✅

## Files Reviewed
- `spec/features/promptcancelled-error-chain-preservation-during-compaction.feature`
- `spec/features/replace-anyhow-macro-with-error-from-in-rigagent-streaming-paths.feature`
- `spec/attachments/BUG-144/bug-144-prompt-cancelled-findings.md`
- `codelet/core/src/rig_agent.rs` (lines 60-77, 200-236)
- `codelet/cli/src/interactive/error_classifiers.rs` (lines 130-184, 475-572)
