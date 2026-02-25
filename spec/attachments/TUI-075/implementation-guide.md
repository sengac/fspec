# TUI-075: Integrate Screen Components into AgentView

## Overview

Replace inline model selector and provider settings implementations in AgentView.tsx with the new screen components. This is the final integration step that removes ~800+ lines from AgentView.tsx.

## Prerequisites

- TUI-072: `useModelSelectorState` hook must be complete
- TUI-073: `ModelSelectorScreen` component must be complete
- TUI-074: `ProviderSettingsScreen` component must be complete
- TUI-076: Type consolidation must be complete

## Changes to AgentView.tsx

### 1. Update Imports

**Remove these imports** (types/functions now in hooks or provider.ts):

```typescript
// REMOVE - types moved to provider.ts
// REMOVE - inline ModelSelection, ProviderSection definitions
// REMOVE - helper functions buildFlatModelList, etc.
```

**Add new imports:**

```typescript
import { ModelSelectorScreen } from './ModelSelectorScreen';
import { ProviderSettingsScreen } from './ProviderSettingsScreen';
import type { ModelSelection } from '../types/provider';
```

### 2. Remove State Declarations (~20 lines, around line 1090-1128)

**REMOVE:**

```typescript
// TUI-034: Model selection state - REMOVE ALL
const [currentModel, setCurrentModel] = useState<ModelSelection | null>(null);
const [providerSections, setProviderSections] = useState<ProviderSection[]>([]);
const [modelsInitialized, setModelsInitialized] = useState(false);
const [showModelSelector, setShowModelSelector] = useState(false);
const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
const [selectedModelIdx, setSelectedModelIdx] = useState(-1);
const [expandedProviders, setExpandedProviders] = useState<Set<string>>(new Set());
const [modelSelectorScrollOffset, setModelSelectorScrollOffset] = useState(0);
const [modelSelectorFilter, setModelSelectorFilter] = useState('');
const [isModelSelectorFilterMode, setIsModelSelectorFilterMode] = useState(false);
const [isRefreshingModels, setIsRefreshingModels] = useState(false);
```

**KEEP only visibility toggles:**

```typescript
// Keep these for controlling which screen is shown
const [showModelSelector, setShowModelSelector] = useState(false);
const [showSettingsTab, setShowSettingsTab] = useState(false);

// Keep currentModel for session creation
const [currentModel, setCurrentModel] = useState<ModelSelection | null>(null);
```

### 3. Remove Type Definitions (~100 lines, around line 248-391)

**REMOVE these type definitions** (moved to provider.ts in TUI-076):

```typescript
// TUI-034: Model selection types
interface ModelSelection { ... }
interface ProviderSection { ... }
type ModelSelectorItem = ...;

// Helper functions
const buildFlatModelList = (...) => { ... };
const flatIndexToSectionModel = (...) => { ... };
const sectionModelToFlatIndex = (...) => { ... };
const mapProviderIdToInternal = (...) => { ... };
const mapInternalToProviderId = (...) => { ... };
const mapModelsDevToRegistryId = (...) => { ... };
```

### 4. Remove Model Selector Input Handling (~200 lines in useInput)

**REMOVE** the entire `if (showModelSelector) { ... }` block from the main `useInput` handler.

Search for patterns like:
```typescript
if (showModelSelector) {
  // ALL THIS CODE MOVES TO ModelSelectorScreen
  if (key.escape) { ... }
  if (key.tab) { ... }
  if (key.upArrow) { ... }
  // etc.
}
```

### 5. Remove Provider Settings Input Handling (~300 lines in useInput)

**REMOVE** the entire `if (showSettingsTab) { ... }` block from the main `useInput` handler.

Search for patterns like:
```typescript
if (showSettingsTab) {
  const currentItem = providerSettings.getCurrentItem();
  // ALL THIS CODE MOVES TO ProviderSettingsScreen
  if (mode.type === 'delete-profile') { ... }
  if (mode.type === 'edit-api-key') { ... }
  // etc.
}
```

### 6. Remove Inline Rendering (~170 lines, around line 7430-7651)

**REMOVE** the inline model selector rendering:

```typescript
// TUI-034: Model selector overlay - REMOVE THIS ENTIRE BLOCK
if (showModelSelector) {
  const modelTextWidth = terminalWidth - 4 - 3;
  return (
    <Box position="absolute" ... >
      ...
    </Box>
  );
}
```

**REMOVE** the inline settings tab rendering:

```typescript
// CONFIG-004 + PROV-007: Settings tab overlay - REMOVE THIS ENTIRE BLOCK
if (showSettingsTab) {
  const settingsVisibleHeightCalc = terminalHeight - 6;
  let effectiveMode: SettingsPanelMode;
  ...
  return (
    <ProviderSettingsPanel ... />
  );
}
```

### 7. Add New Screen Component Rendering

**ADD** simple screen component rendering near the top of the render section:

```tsx
// Model selector screen (full replacement)
if (showModelSelector) {
  return (
    <ModelSelectorScreen
      width={terminalWidth}
      height={terminalHeight}
      currentModelId={currentModel?.apiModelId}
      onSelectModel={(model) => {
        setCurrentModel(model);
        setShowModelSelector(false);
        // Apply model to session if active
        if (currentSessionId) {
          void sessionSetModel(currentSessionId, model.apiModelId);
        }
      }}
      onClose={() => setShowModelSelector(false)}
      onSwitchToSettings={() => {
        setShowModelSelector(false);
        setShowSettingsTab(true);
      }}
    />
  );
}

// Provider settings screen (full replacement)
if (showSettingsTab) {
  return (
    <ProviderSettingsScreen
      width={terminalWidth}
      height={terminalHeight}
      onClose={() => setShowSettingsTab(false)}
      onSwitchToModels={() => {
        setShowSettingsTab(false);
        setShowModelSelector(true);
      }}
    />
  );
}
```

### 8. Update /model and /provider Command Handlers

The command handlers in `handleSubmit` should just toggle visibility:

```typescript
// TUI-034: Handle /model command
if (userMessage === '/model') {
  setInputValue('');
  setShowModelSelector(true);
  return;
}

// CONFIG-004: Handle /provider command
if (userMessage === '/provider') {
  setInputValue('');
  setShowSettingsTab(true);
  return;
}
```

## Verification Checklist

After integration, verify:

- [ ] `/model` command opens ModelSelectorScreen
- [ ] `/provider` command opens ProviderSettingsScreen
- [ ] Tab key switches between model and provider screens
- [ ] Escape closes both screens
- [ ] Model selection works and updates session
- [ ] Provider API key editing works
- [ ] Profile CRUD operations work
- [ ] Filter functionality works in both screens
- [ ] Navigation (arrows, enter) works in both screens
- [ ] TypeScript compiles without errors
- [ ] All existing tests pass

## Expected Results

| Metric | Before | After |
|--------|--------|-------|
| AgentView.tsx lines | ~8000 | ~7200 |
| Lines removed | - | ~800 |
| Inline rendering | 2 blocks | 0 |
| Input handling blocks | 2 large | 0 |

## Rollback Plan

If issues arise, the original code is preserved in git history. Key commits to reference:
- Before TUI-072: Model state extraction
- Before TUI-073: ModelSelectorScreen creation
- Before TUI-074: ProviderSettingsScreen creation
- Before TUI-075: This integration
