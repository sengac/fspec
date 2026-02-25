# AST Research: Profile Model Restoration Bug

## Problem Summary

Profile-based model selection (e.g., Qwen via local vLLM) is saved correctly but NOT restored when starting a new session.

## Root Cause

`modelInitializationService.ts` has its own broken implementation instead of using the correct utilities from `model-selection.ts`.

### Broken Code Location

**File:** `src/tui/services/modelInitializationService.ts`

**Lines 318-325 (broken split):**
```typescript
if (persistedModelString && persistedModelString.includes('/')) {
  const [persistedProviderId, persistedModelId] =
    persistedModelString.split('/');  // BUG: Only gets first part!
  const found = findModelInSections(
    sections,
    persistedProviderId,
    persistedModelId
  );
```

**AST Search Result:**
```
/Users/rquast/projects/fspec/src/tui/services/modelInitializationService.ts:319:7:const [persistedProviderId, persistedModelId] =
```

### Why It's Broken

For a profile model string like `openai:qwen3-coder-next/Qwen/Qwen3-Next-80B`:
- `split('/')` returns `["openai:qwen3-coder-next", "Qwen", "Qwen3-Next-80B"]`
- Destructuring only takes first two: `persistedProviderId = "openai:qwen3-coder-next"`, `persistedModelId = "Qwen"` (WRONG!)
- Should be: `modelId = "Qwen/Qwen3-Next-80B"` (the full path after profile)

**Additionally,** `findModelInSections()` (lines 208-228) only matches by `providerId`:
```typescript
function findModelInSections(
  sections: ProviderSection[],
  providerId: string,
  modelId: string
): { section: ProviderSection; model: NapiModelInfo } | null {
  const section = sections.find(s => s.providerId === providerId);  // Missing profileName check!
```

## Correct Implementation (Already Exists)

**File:** `src/tui/utils/model-selection.ts`

### parseModelString() - Lines 75-106
```typescript
export function parseModelString(modelString: string): ParsedModelString {
  // Check for profile format: 'provider:profile/modelId'
  const colonIndex = modelString.indexOf(':');
  const firstSlashIndex = modelString.indexOf('/');

  if (colonIndex !== -1 && colonIndex < firstSlashIndex) {
    // Profile format: extract provider, profile, and modelId
    const providerId = modelString.substring(0, colonIndex);
    const profileAndModel = modelString.substring(colonIndex + 1);
    const slashIndex = profileAndModel.indexOf('/');
    const profileName = profileAndModel.substring(0, slashIndex);
    const modelId = profileAndModel.substring(slashIndex + 1);
    return { providerId, profileName, modelId };
  }
  // Cloud format: 'provider/modelId'
  ...
}
```

### findSectionForPersistedModel() - Lines 132-153
```typescript
export function findSectionForPersistedModel<T extends ProviderSectionInfo>(
  sections: T[],
  modelString: string
): T | null {
  const { providerId, profileName } = parseModelString(modelString);
  return sections.find(s => {
    if (profileName) {
      // Profile was selected - must match BOTH providerId AND profileName
      return s.providerId === providerId && s.profileName === profileName;
    }
    // Cloud provider - match providerId AND NOT have profileName
    return s.providerId === providerId && !s.profileName;
  }) || null;
}
```

## Fix Required

Replace the broken code in `modelInitializationService.ts` with calls to the correct utilities:

```typescript
import { parseModelString, findSectionForPersistedModel } from '../utils/model-selection';

// Replace lines 318-335 with:
if (persistedModelString) {
  try {
    const parsed = parseModelString(persistedModelString);
    const section = findSectionForPersistedModel(sections, persistedModelString);
    
    if (section) {
      // Find the model within the section
      const model = section.models.find(m => 
        extractModelIdForRegistry(m.id) === extractModelIdForRegistry(parsed.modelId)
      );
      
      if (model) {
        currentModel = createModelSelection(section, model);
        currentProvider = section.internalName;
        persistedModelRestored = true;
      }
    }
  } catch {
    // Invalid model string format - fall back to default
  }
}
```

## Test Gap

The existing tests in `src/utils/__tests__/provider-profiles.test.ts` (lines 495-550) test `findSectionForPersistedModel` directly, which works. However, they don't test the actual code path in `initializeModels()` which uses the broken `findModelInSections()`.

A new test is needed in `src/tui/services/__tests__/modelInitializationService.test.ts` for:
```
Scenario: Restore persisted profile-based model on new session
  Given lastUsedModel is "openai:work-vllm/Qwen/Qwen3-80B"
  And I have a profile "work-vllm" configured for "openai"
  When I call initializeModels()
  Then the restored model should have profileName="work-vllm"
  And the restored model should have modelId containing "Qwen"
```

## AST Search Commands Used

```bash
# Find broken split
ast-grep -p 'const [$A, $B] = $C.split("/")' src/tui/services/modelInitializationService.ts

# Find correct utilities
ast-grep -p 'export function $NAME($$$ARGS): $TYPE { $$$BODY }' src/tui/utils/model-selection.ts
```

## Related Files

| File | Purpose | Status |
|------|---------|--------|
| `src/tui/services/modelInitializationService.ts` | Session model initialization | ❌ BROKEN |
| `src/tui/utils/model-selection.ts` | Model string parsing utilities | ✅ CORRECT |
| `src/tui/services/modelSelectionService.ts` | Model selection + persistence | ✅ CORRECT (uses buildModelString) |
| `src/utils/__tests__/provider-profiles.test.ts` | Profile utility tests | ✅ Tests correct code |
| `src/tui/services/__tests__/modelInitializationService.test.ts` | Init service tests | ❌ Missing profile test |
