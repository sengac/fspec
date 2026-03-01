# PROV-018 Review Findings

## Issues to Fix (In Scope)

### 1. Feature File Step Ordering — Given Steps After Then

4 of 5 scenarios place precondition `And` steps after `Then`, making them read as postconditions. The tests reorder correctly, masking the problem.

**Example (Scenario 1):**
```gherkin
# WRONG — "And models.dev returns..." is a Given, not a Then
Then I should see a Codex (ChatGPT) section with codex models
And models.dev returns OpenAI provider with codex models
```

**Affected scenarios:** 1, 2, 3, 4, 5

**Fix:** Reorder all steps to proper Given/When/Then.

### 2. Coverage `implLines` Point to Wrong Lines

All 5 scenarios link to lines 55-62 which is `extractModelIdForRegistry` (a pre-existing function) plus a JSDoc comment. The actual PROV-018 implementation:

| Function | Lines |
|---|---|
| `isCodexModel` | 65-67 |
| `buildCloudSections` (modifications) | 106-141 |
| `checkCodexOAuthTokens` | 147-154 |
| `extractCodexSection` | 170-218 |

**Fix:** Re-link all 5 scenarios to correct implementation lines.

### 3. Missing Edge Case Test

No test covers: all OpenAI models are codex models → empty OpenAI section should be removed. Code at lines 209-211 handles this, but it's untested.

**Fix:** Add test for this edge case.

### 4. No Estimate Set

Work unit moved to done without estimate despite system reminder.

**Fix:** Set estimate (2 points — single file change, clear scope, well-bounded).

## Pre-existing Issues (Out of Scope)

### `extractModelIdForRegistry` — 3 Divergent Copies

| File | Behavior |
|---|---|
| `modelInitializationService.ts:52` | Strips `-YYYYMMDD` via regex match |
| `useModelSelectorState.ts:123` | Strips `-YYYYMMDD` via regex replace |
| `AgentView.tsx:546` | Keeps Claude dates, strips `-preview-XX-XX` for others |

Copy #3 has intentionally different semantics. This predates PROV-018.

### `AgentView.tsx:1957` Bypasses `buildModelString`

Inline `${currentModel.providerId}/${currentModel.modelId}` instead of `buildModelString()`. Works for non-profile cloud models but is inconsistent with lines 3832 and 4032. Predates PROV-018.
