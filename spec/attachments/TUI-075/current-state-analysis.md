# TUI-075: Current State Analysis

Generated: 2026-02-24

## Executive Summary

The ModelSelectorScreen and ProviderSettingsScreen components have been integrated into AgentView, but there's significant cleanup remaining:

- **AgentView.tsx**: 7455 lines (was 8197, reduced by 742 lines - ~9% reduction)
- **Expected reduction**: ~800+ lines per original plan
- **Remaining work**: Remove orphaned code from incomplete refactoring

## Key Finding: Orphaned Code, Not Missing Wiring

**Line 997 in AgentView.tsx confirms:**
```typescript
// TUI-074: Legacy settings state removed - now managed by useProviderSettingsState in ProviderSettingsScreen
```

The state variables were **intentionally removed** as part of TUI-074, but the code that **called** those state setters wasn't cleaned up. The `useProviderSettingsState` hook now handles everything automatically:
- Provider loading via `reload()` (auto-called on mount)
- `editingApiKey` state management
- `saveApiKey()` and `testConnection()` operations

The orphaned AgentView code is trying to do work that the hook now does automatically.

## What Has Been Done ✅

### 1. New Components Created and Integrated

| Component | Status | Lines |
|-----------|--------|-------|
| `ModelSelectorScreen.tsx` | ✅ Created & integrated | 260 |
| `ModelSelectorView.tsx` | ✅ Created | (presentation) |
| `ProviderSettingsScreen.tsx` | ✅ Created & integrated | 67 |
| `ProviderSettingsPanel.tsx` | ✅ Created | (presentation) |
| `useModelSelectorState.ts` | ✅ Created | 582 |
| `useProviderSettingsState.ts` | ✅ Created | (hook) |
| `useProviderSettingsInput.ts` | ✅ Created | (input handler) |
| `types/provider.ts` | ✅ Created | (consolidated types) |

### 2. AgentView Integration Points

Both screens are being rendered:
```tsx
// Lines 6583-6616 in AgentView.tsx
if (showModelSelector) {
  return <ModelSelectorScreen ... />;
}
if (showSettingsTab) {
  return <ProviderSettingsScreen ... />;
}
```

## What Remains To Be Done ❌

### 1. CRITICAL: Broken/Dead Code with Undefined References

**Problem**: Several state setters are called but NEVER DEFINED:

| Undefined Reference | Locations | Severity |
|---------------------|-----------|----------|
| `setSelectedSettingsIdx` | Lines 2013, 3467, 6594 | 🔴 CRITICAL |
| `setEditingProviderId` | Lines 2014, 3468, 3888, 6595 | 🔴 CRITICAL |
| `setEditingApiKey` | Lines 2015, 3469, 3889, 6596 | 🔴 CRITICAL |
| `setProviderStatuses` | Line 3880 | 🔴 CRITICAL |
| `setConnectionTestResult` | Lines 3890, 3964, 3987, 3999, 4007 | 🔴 CRITICAL |

**Why the build passes**: These are in dead code paths that are never executed at runtime:
1. Lines 2010-2017: Dead code - the slash command handler at line 1994 returns early before reaching this
2. Lines 3467-3472: Inside `handleSubmitWithCommand` but also dead (triggers state instead)
3. Line 6594-6597: Dead code in `onSwitchToSettings` callback

**Action Required**: Remove ALL these broken references.

### 2. Dead Code: `handleSaveApiKey` Function

**Lines 3884-4012**: The entire `handleSaveApiKey` function (~128 lines) is:
- Defined but NEVER called
- References undefined state setters
- Should be completely removed

### 3. Dead Code: `loadProviderStatuses` Usage

**Lines 3860-3881**: The `loadProviderStatuses` callback:
- Is called in multiple places
- Calls `setProviderStatuses(statuses)` - but `providerStatuses` state DOESN'T EXIST
- The calls to `void loadProviderStatuses()` are no-ops

**Action Required**: Remove the function OR fix the state declaration.

### 4. Duplicate Model Loading Logic

**The Problem**: Model loading code exists in TWO places:

#### Location 1: AgentView.tsx (Lines ~1650-1850)
```typescript
// TUI-034: Load models and build provider sections
let allModels: NapiProviderModels[] = [];
allModels = await modelsListAll();
// ... ~200 lines of model loading, section building, persisted model restoration
setProviderSections(sections);
```

#### Location 2: useModelSelectorState.ts (Lines 307-412)
```typescript
const loadModels = useCallback(async () => {
  // ... nearly identical logic
  setProviderSections(sections);
}, []);
```

**Result**: 
- AgentView maintains its own `providerSections` state (line 986)
- `useModelSelectorState` maintains its OWN `providerSections` state (line 214)
- These are NOT synchronized!

**Action Required**: Decide on single source of truth for provider sections.

### 5. Duplicate State in AgentView That Should Be Removed

These states are now managed by hooks but still exist in AgentView:

| State Variable | AgentView Line | Should Be |
|----------------|----------------|-----------|
| `providerSections` | 986-988 | Consider: keep for session creation OR use hook |
| `modelsInitialized` | 989 | Remove (hook manages this) |

### 6. Dead Code: `/provider` and `/model` Command Handlers

**Lines 2001-2017**: These handlers are NEVER REACHED because:
```typescript
// Line 1992-1995 - Early return before command handlers
if (userMessage.startsWith('/') && userMessage.length > 1) {
  executeSlashCommandRef.current?.(userMessage);
  return;  // <-- This returns BEFORE reaching the /model and /provider handlers below
}

// DEAD CODE: These are never reached
if (userMessage === '/model') { ... }    // Line 2001
if (userMessage === '/provider') { ... } // Line 2010
```

**Action Required**: Remove the dead command handlers at lines 2001-2017.

### 7. Callbacks with Broken State References

**`onSwitchToSettings` callback (Lines 6591-6598)**:
```tsx
onSwitchToSettings={() => {
  setShowModelSelector(false);
  setShowSettingsTab(true);
  setSelectedSettingsIdx(0);      // UNDEFINED!
  setEditingProviderId(null);     // UNDEFINED!
  setEditingApiKey('');           // UNDEFINED!
  void loadProviderStatuses();    // Calls setProviderStatuses which is UNDEFINED!
}}
```

**Action Required**: Simplify to just toggle visibility flags:
```tsx
onSwitchToSettings={() => {
  setShowModelSelector(false);
  setShowSettingsTab(true);
}}
```

## Cleanup Checklist

### Phase 1: Remove Dead Code with Undefined References
- [ ] Remove dead `/provider` handler at lines 2010-2017
- [ ] Remove dead `/model` handler at lines 2001-2006
- [ ] Remove `handleSaveApiKey` function (lines 3884-4012)
- [ ] Remove `loadProviderStatuses` function (lines 3860-3881) 
- [ ] Remove `triggerProviderStatusLoad` state and effect (lines 995-996, 5237-5242)

### Phase 2: Fix Broken Callbacks
- [ ] Fix `onSwitchToSettings` callback (line 6591-6598)
- [ ] Fix `handleSubmitWithCommand` `/provider` handler (lines 3464-3472)

### Phase 3: Consolidate State
- [ ] Evaluate if AgentView needs `providerSections` state
- [ ] If yes: remove duplicate loading logic, use hook as source
- [ ] If no: remove state and all 15 references
- [ ] Remove `modelsInitialized` state (line 989) - 6 references to clean

### Phase 4: Verify
- [ ] Build passes: `npm run build`
- [ ] All tests pass: `npm test`
- [ ] `/model` command works
- [ ] `/provider` command works
- [ ] Model selection persists
- [ ] Tab switching between screens works

## Expected Final Result

| Metric | Current | Target |
|--------|---------|--------|
| AgentView.tsx lines | 7455 | ~6800-7000 |
| Dead code functions | 3 | 0 |
| Undefined references | 5 | 0 |
| Duplicate state | providerSections | 0 |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Break model selection | Medium | High | Test thoroughly before/after |
| Break provider settings | Medium | High | Test API key flow end-to-end |
| Break session creation | Low | High | Session still uses AgentView's currentModel |

## Notes

1. The current code "works" only because the broken code paths are never executed
2. The `/provider` and `/model` commands go through `executeSlashCommandRef` → `handleSubmitWithCommand`
3. `ProviderSettingsScreen` manages its own state via `useProviderSettingsState`
4. `ModelSelectorScreen` manages its own state via `useModelSelectorState`
5. AgentView still needs `currentModel` and `setCurrentModel` for session creation
