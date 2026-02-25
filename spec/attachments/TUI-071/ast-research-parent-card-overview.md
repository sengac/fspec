# AST Research: TUI-071 Parent Card Overview

## Summary

TUI-071 is a **parent card** that coordinates 5 child work units. The actual AST research was performed in each child work unit:

| Child | AST Research File |
|-------|-------------------|
| TUI-072 | `spec/attachments/TUI-072/ast-research-model-selector-state.md` |
| TUI-073 | `spec/attachments/TUI-073/ast-research-model-selector.md` |
| TUI-074 | `spec/attachments/TUI-074/ast-research-provider-settings-input.md` |
| TUI-075 | `spec/attachments/TUI-075/ast-research-state-declarations.md` |
| TUI-076 | `spec/attachments/TUI-076/ast-research-type-locations.md` |

## Key Findings (aggregated from children)

### Lines Extracted from AgentView.tsx

1. **Type definitions** (~100 lines) → moved to `src/tui/types/provider.ts`
2. **Model selector state** (~40 lines) → moved to `useModelSelectorState` hook
3. **Model selector helpers** (~100 lines) → moved to `useModelSelectorState` hook
4. **Model selector input handling** (~200 lines) → moved to `ModelSelectorScreen`
5. **Model selector rendering** (~170 lines) → moved to `ModelSelectorScreen`
6. **Provider settings input handling** (~300 lines) → moved to `ProviderSettingsScreen`

**Total: ~800+ lines extracted**

## Architecture After Refactoring

```
AgentView.tsx (reduced)
├── showModelSelector state
├── showSettingsTab state
├── currentModel from Zustand store
├── ModelSelectorScreen component
└── ProviderSettingsScreen component

ModelSelectorScreen.tsx (new)
├── useModelSelectorState() hook
├── ModelSelectorView (presentation)
└── useInput keyboard handler

ProviderSettingsScreen.tsx (new)
├── useProviderSettingsState() hook
├── ProviderSettingsPanel (presentation)
└── useInput keyboard handler

modelStore.ts (new Zustand store)
├── providerSections
├── currentModel
├── modelsInitialized
└── loadModels() action
```

## Verification

- All child work units completed with 100% test coverage
- Build passes successfully
- Tests pass for all new components and hooks
