# TUI-076: Consolidate Provider Types

## Historical Context

### TUI-034: Agent Modal Model Selection (Done)
Introduced the hierarchical model selector with collapsible provider sections. Created:
- `ModelSelection` interface - tracks the currently selected model with all metadata
- `ProviderSection` interface - represents a provider or local profile with its models
- `ModelSelectorItem` type - discriminated union for VirtualList rendering
- Helper functions for flattening/navigating the list

### PROV-007: Provider Configuration Persistence (Implementing)
Extended types with profile support for local servers (vLLM, Ollama):
- Added `profileName` and `profileConfig` to both `ModelSelection` and `ProviderSection`
- Added `isUnreachable` flag for unreachable local servers
- Created `src/tui/types/provider.ts` with settings-related types
- `ProviderSection` exists in BOTH AgentView.tsx AND provider.ts (duplicate!)

### Current Problem
Types are scattered:
- `ModelSelection` - ONLY in AgentView.tsx (line 248)
- `ProviderSection` - DUPLICATED in AgentView.tsx (line 267) AND provider.ts (line 18)
- `ModelSelectorItem` - ONLY in AgentView.tsx (line 286)
- Helper functions - ONLY in AgentView.tsx (lines 301-390)

---

## Scope Definition

### ✅ IN SCOPE for TUI-076

1. **Add `ModelSelection` interface to `src/tui/types/provider.ts`**
   - Copy from AgentView.tsx lines 248-265
   - Add JSDoc comments explaining each field

2. **Add `ModelSelectorItem` type to `src/tui/types/provider.ts`**
   - Copy from AgentView.tsx lines 286-299
   - This is a discriminated union for VirtualList rendering

3. **Verify `ProviderSection` in provider.ts matches AgentView.tsx**
   - Current provider.ts uses `ProfileConfig` type
   - AgentView.tsx uses inline object type `{ baseUrl, apiKey, ... }`
   - Decision: Use `ProfileConfig` (it's the same shape, already imported)

4. **Update imports in AgentView.tsx**
   - Import `ModelSelection`, `ModelSelectorItem`, `ProviderSection` from `../types/provider`
   - Remove duplicate type definitions from AgentView.tsx
   - Keep helper functions in AgentView.tsx (moved in TUI-072)

5. **Verify ModelSelectorView.tsx imports**
   - Already imports `ProviderSection`, `ProviderModel` from `../types/provider` ✓
   - Has local `FlatItem` type that's similar to `ModelSelectorItem` (intentional - different usage)

### ❌ OUT OF SCOPE for TUI-076

| Item | Handled By |
|------|------------|
| Helper functions (`buildFlatModelList`, `flatIndexToSectionModel`, etc.) | TUI-072 |
| State extraction (`selectedSectionIdx`, `expandedProviders`, etc.) | TUI-072 |
| `useModelSelectorState` hook creation | TUI-072 |
| `ModelSelectorScreen` component | TUI-073 |
| `ProviderSettingsScreen` component | TUI-074 |
| Removing code from AgentView.tsx (beyond type definitions) | TUI-075 |
| Input handling extraction | TUI-073, TUI-074 |

---

## Type Definitions to Add

### ModelSelection Interface

```typescript
/**
 * Selected model with full configuration
 * 
 * TUI-034: Created for hierarchical model selector
 * PROV-007: Extended with profileName/profileConfig for local servers
 * 
 * Used to track the currently active model in a session.
 * Persisted to session manifest for resume functionality.
 */
export interface ModelSelection {
  /** Provider ID from models.dev (e.g., "anthropic", "openai", "google") */
  providerId: string;
  
  /** Model ID without provider prefix (e.g., "claude-sonnet-4") */
  modelId: string;
  
  /** Full API model ID for API calls (e.g., "claude-sonnet-4-20250514") */
  apiModelId: string;
  
  /** Human-readable display name (e.g., "Claude Sonnet 4") */
  displayName: string;
  
  /** Whether model supports extended thinking/reasoning */
  reasoning: boolean;
  
  /** Whether model supports vision/image input */
  hasVision: boolean;
  
  /** Context window size in tokens */
  contextWindow: number;
  
  /** Maximum output tokens */
  maxOutput: number;
  
  /** Profile name if model is from a local profile (PROV-007) */
  profileName?: string;
  
  /** Profile config for local servers (PROV-007) */
  profileConfig?: {
    baseUrl: string;
    apiKey: string;
    contextWindow?: number;
    maxOutputTokens?: number;
  };
}
```

### ModelSelectorItem Type

```typescript
/**
 * Flattened item for VirtualList-based model selector
 * 
 * TUI-034: Created for efficient scrolling through hierarchical list
 * 
 * The model selector shows a tree structure:
 * - Provider/Profile sections (collapsible)
 * - Models within each section
 * 
 * This discriminated union allows VirtualList to render both
 * section headers and model items in a flat list.
 */
export type ModelSelectorItem =
  | {
      type: 'section';
      sectionIdx: number;
      section: ProviderSection;
      isExpanded: boolean;
    }
  | {
      type: 'model';
      sectionIdx: number;
      modelIdx: number;
      section: ProviderSection;
      model: NapiModelInfo;
    };
```

---

## Files to Modify

### 1. `src/tui/types/provider.ts`

**Add after existing imports:**
```typescript
// ============================================================================
// Model Selection Types (TUI-034, PROV-007)
// ============================================================================

export interface ModelSelection { ... }
export type ModelSelectorItem = ...;
```

**Keep existing types unchanged:**
- `ProviderModel` (re-export of NapiModelInfo)
- `ProviderSection` (already correct)
- `ProviderStatus`, `ProfileDisplay`, `ProviderWithProfiles` (settings types)
- `SettingsViewMode`, `ProfileFormField`, `PROFILE_FORM_FIELDS`
- `ConnectionTestResult`

### 2. `src/tui/components/AgentView.tsx`

**Update imports (around line 39):**
```typescript
// Before:
import type { ProviderSection as ProviderSectionType, ProviderModel } from '../types/provider';

// After:
import type { 
  ProviderSection, 
  ProviderModel,
  ModelSelection,
  ModelSelectorItem,
} from '../types/provider';
```

**Remove duplicate type definitions (lines 248-299):**
- Remove `interface ModelSelection { ... }`
- Remove `interface ProviderSection { ... }`  
- Remove `type ModelSelectorItem = ...`

**Keep helper functions (lines 301-390):**
These stay in AgentView.tsx for now - they move in TUI-072.

### 3. `src/tui/components/ModelSelectorView.tsx`

**No changes required.**
- Already imports `ProviderSection`, `ProviderModel` from `../types/provider`
- Has its own `FlatItem` type (intentionally different from `ModelSelectorItem`)

### 4. `src/tui/utils/model-selection.ts`

**No changes required.**
- Already imports `ProviderSection` from `../types/provider`
- Helper functions don't need `ModelSelection` or `ModelSelectorItem`

---

## Type Compatibility Notes

### profileConfig Type Alignment

**AgentView.tsx uses inline type:**
```typescript
profileConfig?: {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
};
```

**provider.ts uses imported type:**
```typescript
profileConfig?: ProfileConfig;
```

**ProfileConfig from provider-config.ts:**
```typescript
export interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
}
```

These are structurally identical - TypeScript's structural typing will accept both.

### NapiModelInfo Dependency

`ModelSelectorItem` references `NapiModelInfo` from `@sengac/codelet-napi`:
```typescript
model: NapiModelInfo;
```

This import already exists in provider.ts, so no additional imports needed.

---

## Verification Checklist

- [ ] `npm run build` passes with no type errors
- [ ] `npm test` passes (all existing tests)
- [ ] No duplicate type definitions in AgentView.tsx
- [ ] `ModelSelection` exported from `src/tui/types/provider.ts`
- [ ] `ModelSelectorItem` exported from `src/tui/types/provider.ts`
- [ ] AgentView.tsx imports types from `../types/provider`
- [ ] `grep -r "interface ModelSelection" src/tui` returns only provider.ts
- [ ] `grep -r "type ModelSelectorItem" src/tui` returns only provider.ts

---

## Testing Notes

**Existing tests that should continue passing:**
- `src/tui/__tests__/AgentView-model-selection.test.tsx`
- `src/tui/__tests__/provider-settings-mode-types.test.ts`

**No new tests required for this card.**
This is a pure refactoring task - no behavior changes, just moving types.

---

## Dependencies

**This card (TUI-076) has NO dependencies.**

**Cards that depend on this:**
- TUI-072: Will import `ModelSelection`, `ModelSelectorItem` from provider.ts
- TUI-073: Will import `ModelSelection`, `ModelSelectorItem` from provider.ts
- TUI-075: Will verify all types come from single source
