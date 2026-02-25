# TUI-071: Implementation Order

## Overview

This document outlines the execution order for extracting `/provider` and `/model` screens from `AgentView.tsx`. The refactoring is broken into 5 child work units with explicit dependencies.

## Dependency Graph

```
                    ┌─────────────────────────────────────────┐
                    │             TUI-071 (Parent)            │
                    │    Model Selector Component Extraction   │
                    └─────────────────────────────────────────┘
                                        │
          ┌─────────────────────────────┼─────────────────────────────┐
          │                             │                             │
          ▼                             ▼                             ▼
┌───────────────────┐       ┌───────────────────┐       ┌───────────────────┐
│     TUI-072       │       │     TUI-074       │       │     TUI-076       │
│ useModelSelector  │       │ ProviderSettings  │       │   Consolidate     │
│   State (hook)    │       │ Screen (component)│       │  provider types   │
│                   │       │                   │       │                   │
│ No dependencies   │       │ No dependencies   │       │ No dependencies   │
└─────────┬─────────┘       └─────────┬─────────┘       └─────────┬─────────┘
          │                           │                           │
          ▼                           │                           │
┌───────────────────┐                 │                           │
│     TUI-073       │                 │                           │
│ ModelSelector     │                 │                           │
│ Screen (component)│                 │                           │
│                   │                 │                           │
│ Depends: TUI-072  │                 │                           │
└─────────┬─────────┘                 │                           │
          │                           │                           │
          └───────────────────────────┼───────────────────────────┘
                                      │
                                      ▼
                          ┌───────────────────┐
                          │     TUI-075       │
                          │    Integrate      │
                          │  into AgentView   │
                          │                   │
                          │ Depends: TUI-073  │
                          │          TUI-074  │
                          │          TUI-076  │
                          └───────────────────┘
```

## Execution Phases

### Phase 1: Foundation (Parallel - No Dependencies)

These three work units can be done in parallel or in any order:

| Order | Work Unit | Title | Est. Effort | Key Deliverable |
|-------|-----------|-------|-------------|-----------------|
| 1a | **TUI-076** | Consolidate provider types | Small | `src/tui/types/provider.ts` updated |
| 1b | **TUI-072** | Create useModelSelectorState hook | Medium | `src/tui/hooks/useModelSelectorState.ts` |
| 1c | **TUI-074** | Create ProviderSettingsScreen | Medium | `src/tui/components/ProviderSettingsScreen.tsx` |

**Recommendation**: Start with TUI-076 (types) as it's the smallest and other work will import from it.

### Phase 2: Model Screen (Depends on TUI-072)

| Order | Work Unit | Title | Est. Effort | Key Deliverable |
|-------|-----------|-------|-------------|-----------------|
| 2 | **TUI-073** | Create ModelSelectorScreen | Medium | `src/tui/components/ModelSelectorScreen.tsx` |

**Must wait for**: TUI-072 (the hook it uses)

### Phase 3: Integration (Depends on All)

| Order | Work Unit | Title | Est. Effort | Key Deliverable |
|-------|-----------|-------|-------------|-----------------|
| 3 | **TUI-075** | Integrate into AgentView | Large | AgentView.tsx reduced by ~800 lines |

**Must wait for**: TUI-073, TUI-074, TUI-076

---

## Recommended Sequential Order

If doing work units one at a time, follow this order:

### Step 1: TUI-076 - Consolidate Provider Types
**File**: `src/tui/types/provider.ts`

1. Add `ModelSelection` interface
2. Add `ModelSelectorItem` type  
3. Verify `ProviderSection` matches AgentView usage
4. Run `npm run build` to verify types compile

### Step 2: TUI-072 - Create useModelSelectorState Hook
**File**: `src/tui/hooks/useModelSelectorState.ts`

1. Create hook following `useProviderSettingsState.ts` pattern
2. Extract state declarations from AgentView.tsx (lines ~1090-1128)
3. Extract helper functions (lines ~302-391)
4. Add model loading logic
5. Write unit tests
6. Run `npm test` to verify

### Step 3: TUI-074 - Create ProviderSettingsScreen
**File**: `src/tui/components/ProviderSettingsScreen.tsx`

1. Create component shell
2. Import existing `useProviderSettingsState` hook
3. Move ~300 lines of input handling from AgentView (lines ~6857-7155)
4. Render `ProviderSettingsPanel` 
5. Write unit tests
6. Run `npm test` to verify

### Step 4: TUI-073 - Create ModelSelectorScreen
**File**: `src/tui/components/ModelSelectorScreen.tsx`

1. Create component shell
2. Import `useModelSelectorState` hook (from step 2)
3. Move input handling from AgentView
4. Decide: modify `ModelSelectorView.tsx` to be purely presentational OR use as-is
5. Write unit tests
6. Run `npm test` to verify

### Step 5: TUI-075 - Integrate into AgentView
**File**: `src/tui/components/AgentView.tsx`

1. Add imports for new screen components
2. Remove type definitions (~100 lines)
3. Remove state declarations (keep only `showModelSelector`, `showSettingsTab`, `currentModel`)
4. Remove model selector input handling (~200 lines)
5. Remove provider settings input handling (~300 lines)
6. Remove inline rendering (~170 lines)
7. Add simple screen component rendering
8. Run full test suite
9. Manual testing of `/model` and `/provider` commands

---

## Verification Checklist

After completing all work units:

- [ ] `npm run build` passes
- [ ] `npm test` passes (all tests)
- [ ] `/model` command opens model selector
- [ ] `/provider` command opens provider settings
- [ ] Tab switches between model and provider screens
- [ ] Escape closes both screens
- [ ] Model selection updates session
- [ ] Provider API key editing works
- [ ] Profile CRUD operations work
- [ ] Filter works in both screens
- [ ] Arrow navigation works in both screens
- [ ] AgentView.tsx reduced by ~800 lines

---

## Risk Mitigation

### Rollback Points
Create git commits after each work unit:
- `git commit -m "TUI-076: Consolidate provider types"`
- `git commit -m "TUI-072: Create useModelSelectorState hook"`
- `git commit -m "TUI-074: Create ProviderSettingsScreen"`
- `git commit -m "TUI-073: Create ModelSelectorScreen"`
- `git commit -m "TUI-075: Integrate screen components into AgentView"`

### Testing Strategy
- Unit tests for each new hook/component
- Integration tests for screen interactions
- Manual smoke test after TUI-075 integration

### If Something Breaks
1. Check git diff to see what changed
2. Run `npm test` to identify failing tests
3. Use `git checkout` to rollback specific files
4. Reference the implementation guide attachments for correct code

---

## Time Estimates

| Work Unit | Estimated Time | Complexity |
|-----------|---------------|------------|
| TUI-076 | 30 min - 1 hr | Low |
| TUI-072 | 1 - 2 hrs | Medium |
| TUI-074 | 1 - 2 hrs | Medium |
| TUI-073 | 1 - 2 hrs | Medium |
| TUI-075 | 2 - 3 hrs | High (integration) |
| **Total** | **5 - 10 hrs** | |

---

## Files Created/Modified Summary

### New Files
- `src/tui/hooks/useModelSelectorState.ts` (TUI-072)
- `src/tui/components/ModelSelectorScreen.tsx` (TUI-073)
- `src/tui/components/ProviderSettingsScreen.tsx` (TUI-074)

### Modified Files
- `src/tui/types/provider.ts` (TUI-076)
- `src/tui/components/AgentView.tsx` (TUI-075)
- `src/tui/components/ModelSelectorView.tsx` (TUI-073 - may need changes)

### Files Unchanged
- `src/tui/hooks/useProviderSettingsState.ts` (already exists, reused)
- `src/tui/components/ProviderSettingsPanel.tsx` (already exists, reused)
