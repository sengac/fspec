# Model Selection Architecture Analysis

**Work Unit:** PROV-008  
**Date:** 2026-02-25  
**Status:** Analysis Complete (Verified)

---

## Executive Summary

The model selection system has one primary issue: **duplicated TypeScript code** in `AgentView.tsx`. The Rust layer is already well-designed and handles profile models correctly. This is a focused TypeScript refactoring task.

---

## 1. Root Cause: The Warning Noise - ✅ ALREADY RESOLVED

The original complaint about warnings like:
```
[RUST:WARN] parse_model_string: provider 'Qwen' not in registry
```

**This is NOT an issue in the current codebase.**

The `selected_model_id()` method in `manager.rs` (lines 287-302) silently catches errors from `parse_model_string` and falls through to return the model string directly:

```rust
pub fn selected_model_id(&self) -> Option<String> {
    let model_string = self.selected_model.as_ref()?;

    // If we have a registry, try to look up the model
    if let Some(registry) = self.model_registry.as_ref() {
        if let Ok((provider_id, model_id)) = registry.parse_model_string(model_string) {
            if let Ok(model_info) = registry.get_model(&provider_id, &model_id) {
                return Some(model_info.id.clone());
            }
        }
    }

    // No registry or lookup failed - return the stored string directly
    Some(model_string.clone())
}
```

**No Rust changes are needed.**

---

## 2. The Actual Problem: Duplicate TypeScript Handlers

### Location: `src/tui/components/AgentView.tsx`

Two nearly identical callback functions exist:

| Function | Lines | Status | Input Type |
|----------|-------|--------|------------|
| `handleModelSelect` | 3601-3665 | ✅ ACTIVE | `ModelSelection` |
| `handleSelectModel` | 3669-3747 | ❌ DEPRECATED & UNUSED | `ProviderSection, NapiModelInfo` |

**Verification:** `handleSelectModel` is defined but **never called anywhere**:
```bash
grep -r "handleSelectModel[^e]" src/
# Only result: the definition itself on line 3669
```

### Duplicated Logic (78 lines each):

Both handlers do exactly the same thing:
1. **Build model string** (lines 3604-3607 vs 3671-3673)
2. **Set env vars for profile** (lines 3612-3625 vs 3678-3694) - **14 identical lines**
3. **Update Rust session** (lines 3627-3640 vs 3696-3709) - **13 identical lines**
4. **Set local state** (lines 3642-3645 vs 3711-3726)
5. **Persist to config** (lines 3648-3662 vs 3729-3744) - **14 identical lines**

---

## 3. What Actually Needs To Be Done

### Phase 1: Delete Dead Code (5 minutes)
- **Delete** `handleSelectModel` (lines 3667-3747) - 80 lines of unused code

### Phase 2: Extract Profile Environment Service (30 minutes)
**Create:** `src/tui/services/profileEnvironmentService.ts`

```typescript
import type { ProfileConfig } from '../store/modelStore';

/**
 * Configure environment variables for profile-based models.
 * 
 * Called before any Rust session operations to ensure
 * OPENAI_BASE_URL and OPENAI_API_KEY are set correctly.
 */
export function configureProfileEnvironment(config: ProfileConfig): void {
  process.env.OPENAI_BASE_URL = config.baseUrl;
  process.env.OPENAI_API_KEY = config.apiKey;
  
  if (config.contextWindow) {
    process.env.OPENAI_CONTEXT_WINDOW = String(config.contextWindow);
  }
  if (config.maxOutputTokens) {
    process.env.OPENAI_MAX_OUTPUT_TOKENS = String(config.maxOutputTokens);
  }
}
```

### Phase 3: Extract Model Selection Service (1 hour)
**Create:** `src/tui/services/modelSelectionService.ts`

```typescript
import { sessionSetModel, sessionSetModelProfile } from '@sengac/codelet-napi';
import { useModelStore, type ModelSelection } from '../store/modelStore';
import { loadConfig, writeConfig } from '../../utils/config';
import { buildModelString } from '../../utils/provider-config';
import { configureProfileEnvironment } from './profileEnvironmentService';
import { logger } from '../../utils/logger';

export interface SelectModelOptions {
  sessionId: string | null;
  selection: ModelSelection;
  onRefreshRustState?: (sessionId: string) => void;
  onSetCurrentModel?: (selection: ModelSelection) => void;
  onSetCurrentProvider?: (provider: string) => void;
}

export async function selectModel(options: SelectModelOptions): Promise<void> {
  const { sessionId, selection, onRefreshRustState, onSetCurrentModel, onSetCurrentProvider } = options;
  
  // 1. Configure environment for profile-based models
  if (selection.profileConfig) {
    configureProfileEnvironment(selection.profileConfig);
  }
  
  // 2. Update Rust session if exists
  if (sessionId) {
    try {
      if (selection.profileConfig) {
        await sessionSetModelProfile(sessionId, selection.providerId, selection.modelId);
      } else {
        await sessionSetModel(sessionId, selection.providerId, selection.modelId);
      }
      onRefreshRustState?.(sessionId);
    } catch (err) {
      logger.error('Failed to update background session model', { error: err });
    }
  } else {
    // No session - store for later sync
    onSetCurrentModel?.(selection);
    onSetCurrentProvider?.(mapProviderIdToInternal(selection.providerId));
  }
  
  // 3. Persist to config
  try {
    const modelString = buildModelString(
      { providerId: selection.providerId, profileName: selection.profileName },
      selection.modelId
    );
    const existingConfig = await loadConfig();
    await writeConfig('user', {
      ...existingConfig,
      tui: { ...existingConfig?.tui, lastUsedModel: modelString },
    });
  } catch (err) {
    logger.error('Failed to persist model selection', { error: err });
  }
}
```

### Phase 4: Simplify handleModelSelect (30 minutes)

After extraction, `handleModelSelect` becomes:

```typescript
const handleModelSelect = useCallback(
  async (selection: ModelSelection) => {
    setShowModelSelector(false);
    
    await selectModel({
      sessionId: currentSessionId,
      selection,
      onRefreshRustState: refreshRustState,
      onSetCurrentModel: setCurrentModel,
      onSetCurrentProvider: setCurrentProvider,
    });
  },
  [currentSessionId]
);
```

**Reduction:** From 64 lines to 12 lines.

---

## 4. Files Requiring Changes

| File | Action | Lines Changed |
|------|--------|---------------|
| `src/tui/components/AgentView.tsx` | Delete `handleSelectModel`, simplify `handleModelSelect` | -80, -52 |
| `src/tui/services/profileEnvironmentService.ts` | **CREATE** | +20 |
| `src/tui/services/modelSelectionService.ts` | **CREATE** | +60 |
| `src/tui/services/index.ts` | Export new services | +2 |

**Net change:** ~-50 lines (deletion of duplicates)

---

## 5. What Does NOT Need To Change

| File | Reason |
|------|--------|
| `codelet/providers/src/manager.rs` | Already handles profile models correctly |
| `codelet/providers/src/models/registry.rs` | Error handling is appropriate |
| Model store types | Already has `ProfileConfig` |

---

## 6. Testing Strategy

### Unit Tests Needed:
1. `profileEnvironmentService.test.ts`
   - Test env var setup with full config
   - Test env var setup with partial config (no contextWindow)
   - Test env var setup overwrites previous values

2. `modelSelectionService.test.ts`
   - Test cloud provider selection with session
   - Test profile model selection with session
   - Test selection without session (stores for later)
   - Test config persistence
   - Test error handling

### Integration Test:
- Verify `handleModelSelect` still works after refactor
- Switch between cloud and profile models

---

## 7. Acceptance Criteria

1. ✅ `handleSelectModel` callback is deleted from `AgentView.tsx`
2. ✅ `profileEnvironmentService.ts` exists and is exported
3. ✅ `modelSelectionService.ts` exists and is exported
4. ✅ `handleModelSelect` delegates to the new service
5. ✅ TypeScript compiles without errors
6. ✅ All existing model selection behavior works unchanged
7. ✅ Unit tests pass for new services

---

## Appendix: Verified Line Numbers (as of 2026-02-25)

| Item | File | Lines |
|------|------|-------|
| `handleModelSelect` | `AgentView.tsx` | 3601-3665 |
| `handleSelectModel` (deprecated) | `AgentView.tsx` | 3669-3747 |
| Env var setup (dup 1) | `AgentView.tsx` | 3612-3625 |
| Env var setup (dup 2) | `AgentView.tsx` | 3678-3694 |
| `selected_model_id()` | `manager.rs` | 287-302 |
| `parse_model_string()` | `registry.rs` | 51-75 |
| `set_model_direct()` | `manager.rs` | 265-279 |
