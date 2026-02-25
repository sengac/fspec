# AST Research: Duplicate Patterns in AgentView.tsx

**Work Unit:** PROV-008  
**Date:** 2026-02-25  
**Tool:** AstGrep

## Summary

AST analysis confirms the duplicate code patterns identified in the architecture analysis. The same operations appear in multiple locations, violating DRY principles.

---

## 1. Environment Variable Assignments (process.env.$VAR = $VALUE)

**Pattern:** `process.env.$VAR = $VALUE`

| Location | Line | Code |
|----------|------|------|
| handleModelSelect | 3613 | `process.env.OPENAI_BASE_URL = selection.profileConfig.baseUrl` |
| handleModelSelect | 3614 | `process.env.OPENAI_API_KEY = selection.profileConfig.apiKey` |
| handleModelSelect | 3616 | `process.env.OPENAI_CONTEXT_WINDOW = String(...)` |
| handleModelSelect | 3621 | `process.env.OPENAI_MAX_OUTPUT_TOKENS = String(...)` |
| handleSelectModel | 3679 | `process.env.OPENAI_BASE_URL = section.profileConfig.baseUrl` |
| handleSelectModel | 3680 | `process.env.OPENAI_API_KEY = section.profileConfig.apiKey` |
| handleSelectModel | 3682 | `process.env.OPENAI_CONTEXT_WINDOW = String(...)` |
| handleSelectModel | 3687 | `process.env.OPENAI_MAX_OUTPUT_TOKENS = String(...)` |
| createNewSession | 4324 | `process.env.OPENAI_BASE_URL = currentModel.profileConfig.baseUrl` |
| createNewSession | 4325 | `process.env.OPENAI_API_KEY = currentModel.profileConfig.apiKey` |
| createNewSession | 4327 | `process.env.OPENAI_CONTEXT_WINDOW = String(...)` |
| createNewSession | 4332 | `process.env.OPENAI_MAX_OUTPUT_TOKENS = String(...)` |
| handleResumeSession | 4522 | `process.env.OPENAI_BASE_URL = currentModel.profileConfig.baseUrl` |
| handleResumeSession | 4523 | `process.env.OPENAI_API_KEY = currentModel.profileConfig.apiKey` |
| handleResumeSession | 4525 | `process.env.OPENAI_CONTEXT_WINDOW = String(...)` |
| handleResumeSession | 4530 | `process.env.OPENAI_MAX_OUTPUT_TOKENS = String(...)` |

**Analysis:** 16 occurrences across 4 functions. All follow identical pattern - extractable to `configureProfileEnvironment()`.

---

## 2. Session Model Updates (sessionSetModel/sessionSetModelProfile)

**Pattern:** `await sessionSetModel($$$ARGS)` / `await sessionSetModelProfile($$$ARGS)`

| Function | Type | Line | Code |
|----------|------|------|------|
| handleModelSelect | sessionSetModelProfile | 3631 | `await sessionSetModelProfile(currentSessionId, selection.providerId, selection.modelId)` |
| handleModelSelect | sessionSetModel | 3633 | `await sessionSetModel(currentSessionId, selection.providerId, selection.modelId)` |
| handleSelectModel | sessionSetModelProfile | 3700 | `await sessionSetModelProfile(currentSessionId, section.providerId, modelId)` |
| handleSelectModel | sessionSetModel | 3702 | `await sessionSetModel(currentSessionId, section.providerId, modelId)` |

**Analysis:** 4 occurrences across 2 functions. Decision logic (which function to call based on profileConfig) is duplicated.

---

## 3. Config Persistence (writeConfig)

**Pattern:** `await writeConfig($$$ARGS)`

| Function | Line | Code |
|----------|------|------|
| handleModelSelect | 3657 | `await writeConfig('user', updatedConfig)` |
| handleSelectModel | 3738 | `await writeConfig('user', updatedConfig)` |

**Analysis:** 2 occurrences with identical surrounding logic (load existing, merge, write).

---

## 4. handleSelectModel Usage Analysis

**Conclusion:** `handleSelectModel` (lines 3669-3747) is defined but NEVER called.

Verified by searching for:
- `handleSelectModel(` - only the definition itself
- No references in JSX
- No references in other callbacks

**Recommendation:** Safe to delete - dead code.

---

## Refactoring Strategy

### New Service: profileEnvironmentService.ts
Extracts: 16 env var assignments → 1 function call per location

### New Service: modelSelectionService.ts  
Extracts: Session updates + config persistence logic

### Deletion: handleSelectModel
Removes: 80 lines of unused code

### Refactored: handleModelSelect
From: 64 lines → To: ~12 lines (delegating to service)

---

## Net Code Reduction

| Category | Before | After | Delta |
|----------|--------|-------|-------|
| Env var duplication | 16 occurrences | 4 calls | -12 sites |
| Session update logic | 2 duplicates | 1 service | -1 duplicate |
| Config persistence | 2 duplicates | 1 service | -1 duplicate |
| Dead code | 80 lines | 0 | -80 lines |
| **handleModelSelect** | 64 lines | 12 lines | -52 lines |
