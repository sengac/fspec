# AST Research: ModelSelectorScreen Component Extraction

## Overview

Analysis of code that needs to be extracted from AgentView.tsx and ModelSelectorView.tsx into the new ModelSelectorScreen component.

## ModelSelectorView.tsx Analysis

### useInput Handler to Remove

**Location**: Line 250-363 (114 lines)

```typescript
// @step Found useInput call at line 250
useInput(
  (input, key) => {
    // Filter mode handling (lines 253-278)
    // Close on Escape (lines 280-288)
    // Tab to switch to settings (lines 291-294)
    // Filter mode entry (lines 297-300)
    // Refresh (lines 303-306)
    // Navigation (lines 309-316)
    // Enter to select/expand (lines 319-331)
    // Left/Right arrows (lines 334-360)
  },
  { isActive: true }
);
```

### useState Calls to Remove

| Line | State Variable | Purpose |
|------|----------------|---------|
| 102 | selectedSectionIdx | Track selected section |
| 103 | selectedModelIdx | Track selected model (-1 = header) |
| 106-108 | expandedSections | Set of expanded section indices |
| 111 | filter | Filter string |
| 112 | isFilterMode | Whether in filter mode |
| 115 | scrollOffset | Scroll position |

### Props Interface Changes

**Current Props** (lines 23-42):
```typescript
interface ModelSelectorViewProps {
  width: number;
  height: number;
  sections: ProviderSection[];
  currentModelId?: string;
  isRefreshing: boolean;
  onClose: () => void;            // REMOVE
  onSwitchToSettings: () => void; // REMOVE
  onSelectModel: (section, model) => void; // REMOVE
  onRefresh: () => void;          // REMOVE
}
```

**New Props** (pure presentation):
```typescript
interface ModelSelectorViewProps {
  width: number;
  height: number;
  sections: ProviderSection[];
  flatItems: ModelSelectorItem[];
  selectedSectionIdx: number;
  selectedModelIdx: number;
  expandedProviders: Set<string>;
  scrollOffset: number;
  visibleHeight: number;
  filter: string;
  isFilterMode: boolean;
  currentModelId?: string;
  isRefreshing: boolean;
}
```

## AgentView.tsx Analysis

### State Declarations to Keep

**Line 1050**: `const [showModelSelector, setShowModelSelector] = useState(false);`

This controls visibility and MUST remain in AgentView.

### Keyboard Handling to Remove

**Lines 6641-6808**: Model selector keyboard block (~167 lines)

```typescript
// @step Found keyboard handling at line 6641
if (showModelSelector) {
  // Filter mode handling (6643-6668)
  // Escape handling (6671-6678)
  // Filter mode entry (6681-6684)
  // Left arrow: collapse (6687-6700)
  // Right arrow: expand (6704-6715)
  // Up arrow: navigate (6718-6741)
  // Down arrow: navigate (6745-6763)
  // Enter: select/toggle (6766-6788)
  // Refresh (6792-6795)
  // Tab: settings (6798-6806)
}
```

### Mouse Handling to Remove

**Lines 6350-6357**: Mouse scroll for model selector

```typescript
// @step Found mouse handling at line 6350
if (showModelSelector) {
  if (key.mouse.button === 'wheelUp') {
    navigateModelSelectorByDelta(-1);
    return true;
  } else if (key.mouse.button === 'wheelDown') {
    navigateModelSelectorByDelta(1);
    return true;
  }
}
```

### Inline Rendering to Remove

**Lines 7384-7548**: Model selector rendering (~165 lines)

```typescript
// @step Found rendering at line 7384
if (showModelSelector) {
  // Full overlay rendering
  // Header with "Select Model"
  // Filter input
  // Scrollable list
  // Scrollbar
  // Footer keybindings
}
```

## useModelSelectorState Hook API

The hook at `src/tui/hooks/useModelSelectorState.ts` provides:

### State
- `currentModel: ModelSelection | null`
- `providerSections: ProviderSection[]`
- `flatItems: ModelSelectorItem[]`
- `filteredFlatItems: ModelSelectorItem[]`
- `selectedSectionIdx: number`
- `selectedModelIdx: number`
- `expandedProviders: Set<string>`
- `scrollOffset: number`
- `visibleHeight: number`
- `filter: string`
- `isFilterMode: boolean`
- `isVisible: boolean`
- `isLoading: boolean`
- `isRefreshing: boolean`
- `modelsInitialized: boolean`

### Actions
- `setCurrentModel(model: ModelSelection | null)`
- `setSelectedSectionIdx(idx: number)`
- `setSelectedModelIdx(idx: number)`
- `setScrollOffset(offset: number)`
- `setVisibleHeight(height: number)`
- `setFilter(filter: string)`
- `setIsFilterMode(mode: boolean)`
- `setIsVisible(visible: boolean)`

### Operations
- `toggleSectionExpansion(providerId: string)`
- `refreshModels(): Promise<void>`
- `loadModels(): Promise<void>`
- `selectModel(section, model): ModelSelection`

### Navigation
- `navigateUp()`
- `navigateDown()`
- `getCurrentFlatIndex(): number`

## Expected Changes Summary

| Component | Lines Removed | Lines Added | Net Change |
|-----------|---------------|-------------|------------|
| AgentView.tsx | ~350 | ~15 | -335 |
| ModelSelectorView.tsx | ~125 | ~10 | -115 |
| ModelSelectorScreen.tsx | 0 | ~150 | +150 |
| **Total** | ~475 | ~175 | **-300** |
