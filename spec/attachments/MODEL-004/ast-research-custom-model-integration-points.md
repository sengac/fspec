# AST Research: Custom Model Integration Points

**Date:** 2026-04-11
**Work Unit:** MODEL-004
**Method:** AstGrep pattern search on TypeScript source files

## 1. ProfileConfig Interface (Extension Point)

**File:** `src/utils/provider-config.ts:42`
**Pattern:** `interface ProfileConfig { $$$FIELDS }`

```typescript
interface ProfileConfig {
  baseUrl: string;
  apiKey: string;
  contextWindow?: number;
  maxOutputTokens?: number;
}
```

**MODEL-004 Action:** Add `customModels?: CustomModelDefinition[]` field.

## 2. ModelSelection Interface (Extension Point)

**File:** `src/tui/types/provider.ts:45`
**Pattern:** `interface ModelSelection { $$$FIELDS }`

```typescript
interface ModelSelection {
  providerId: string;
  modelId: string;
  apiModelId: string;
  displayName: string;
  reasoning: boolean;
  hasVision: boolean;
  contextWindow: number;
  maxOutput: number;
  profileName?: string;
  profileConfig?: ProfileConfig;
}
```

**MODEL-004 Action:** Add `facade?: string` field.

## 3. loadProfileSections Function (Merge Point)

**File:** `src/tui/services/modelInitializationService.ts:264`
**Pattern:** `async function loadProfileSections()`

This function iterates OpenAI profiles, calls `modelsListLocalOpenai()`, and builds `NapiModelInfo[]`.
Custom models need to be loaded from config and merged here.

**MODEL-004 Action:** After the `modelsListLocalOpenai()` call, load custom models from profile config and merge into `localModels[]`. Update `isUnreachable` logic.

## 4. selectModel Function (Facade Propagation Point)

**File:** `src/tui/services/modelSelectionService.ts:71`

The `selectModel` function calls `sessionSetModelProfile(sessionId, providerId, modelId)`.
MODEL-004 needs to add the optional `facadeOverride` parameter.

## 5. ModelSelectorView Badge Rendering

**File:** `src/tui/components/ModelSelectorView.tsx`
**File:** `src/tui/components/ModelSelectorScreen.tsx`

These are the TUI components that need `[C]` badge rendering and `a/e/d` keybind handling.

## 6. NAPI Binding - sessionSetModelProfile

**File:** `codelet/napi/src/session_manager.rs:6462-6483`

Currently accepts `(session_id, provider_id, model_id)`. Needs optional `facade_override` param.
**BLOCKED on MODEL-005** — Rust changes depend on MODEL-005 landing first.

## 7. ProviderManager - set_model_direct

**File:** `codelet/providers/src/manager.rs:281-299`

Needs `facade_override: Option<String>` field and storage on ProviderManager struct.
**BLOCKED on MODEL-005** — Rust changes depend on MODEL-005 landing first.
