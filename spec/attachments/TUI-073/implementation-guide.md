# TUI-073: Create ModelSelectorScreen Component

## Overview

Create an **orchestrator component** that:
1. Wraps `ModelSelectorView` (presentation component)
2. Uses `useModelSelectorState` hook for state management (TUI-072, DONE)
3. Handles ALL keyboard input via `useInput`
4. Delegates to parent callbacks: `onSelectModel`, `onClose`, `onSwitchToSettings`

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ ModelSelectorScreen (NEW: src/tui/components/)             │
│   ├── useModelSelectorState()  ← state & operations        │
│   ├── useInput()               ← keyboard handling         │
│   └── <ModelSelectorView />    ← presentation only         │
└─────────────────────────────────────────────────────────────┘

AgentView.tsx
  └── showModelSelector ? <ModelSelectorScreen ... /> : null
```

## Dependencies (All Complete ✅)

| Dependency | Status | Description |
|------------|--------|-------------|
| TUI-072 | ✅ DONE | `useModelSelectorState` hook at `src/tui/hooks/useModelSelectorState.ts` |
| TUI-076 | ✅ DONE | Consolidated types at `src/tui/types/provider.ts` |

---

## Current State Analysis

### AgentView.tsx - Code to DELETE

#### 1. State Declarations (Lines 1045-1059, ~15 variables)

```typescript
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
```

**Note**: `showModelSelector` and `setShowModelSelector` should REMAIN in AgentView (controls visibility).

#### 2. Keyboard Handling (Lines 6641-6808, ~170 lines)

```typescript
// TUI-034: Model selector keyboard handling
if (showModelSelector) {
  // Filter mode handling (lines 6643-6668)
  if (isModelSelectorFilterMode) {
    if (key.escape) { /* clear filter, exit mode */ }
    if (key.return) { /* exit mode, keep filter */ }
    if (key.backspace || key.delete) { /* remove last char */ }
    // Accept printable characters
  }

  // Escape handling (lines 6671-6678)
  if (key.escape) {
    if (modelSelectorFilter) { setModelSelectorFilter(''); return; }
    setShowModelSelector(false);
    return;
  }

  // Filter mode entry (line 6681-6684)
  if (input === '/') { setIsModelSelectorFilterMode(true); return; }

  // Left arrow: collapse section (lines 6687-6700)
  if (key.leftArrow) {
    const currentSection = providerSections[selectedSectionIdx];
    if (currentSection && expandedProviders.has(currentSection.providerId)) {
      // Remove from expanded, move to section header
    }
  }

  // Right arrow: expand section (lines 6704-6715)
  if (key.rightArrow) {
    const currentSection = providerSections[selectedSectionIdx];
    if (currentSection && !expandedProviders.has(currentSection.providerId)) {
      // Add to expanded
    }
  }

  // Up arrow: navigate up (lines 6718-6741)
  if (key.upArrow) { /* complex navigation through sections/models */ }

  // Down arrow: navigate down (lines 6745-6763)
  if (key.downArrow) { /* complex navigation through sections/models */ }

  // Enter: select model or toggle section (lines 6766-6788)
  if (key.return) {
    if (selectedModelIdx === -1) {
      // Toggle section expansion
    } else {
      // Select model
      void handleSelectModel(currentSection, model);
    }
  }

  // Refresh (lines 6792-6795)
  if ((input === 'r' || input === 'R') && !isRefreshingModels) {
    void refreshModels();
  }

  // Tab: switch to settings (lines 6798-6806)
  if (key.tab) {
    setShowModelSelector(false);
    setShowSettingsTab(true);
    // ... more setup
  }
}
```

#### 3. Mouse Scroll Handling (Lines 6350-6357)

```typescript
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

#### 4. Inline Rendering (Lines 7384-7548, ~165 lines)

```tsx
// TUI-034: Model selector overlay (hierarchical with collapsible sections)
if (showModelSelector) {
  const modelTextWidth = terminalWidth - 4 - 3;
  return (
    <Box position="absolute" flexDirection="column" width={terminalWidth} height={terminalHeight}>
      {/* Header with "Select Model" title */}
      {/* Filter input box */}
      {/* Scrollable list with sections and models */}
      {/* Scrollbar */}
      {/* Footer with keybindings */}
    </Box>
  );
}
```

---

### ModelSelectorView.tsx - Code to MODIFY

**File**: `src/tui/components/ModelSelectorView.tsx` (525 lines)

#### useInput Handler to REMOVE (Lines 250-363)

```typescript
useInput(
  (input, key) => {
    // Filter mode handling (lines 253-278)
    if (isFilterMode) {
      if (key.escape) { /* ... */ }
      if (key.return) { /* ... */ }
      if (key.backspace || key.delete) { /* ... */ }
      // Accept printable characters
    }

    // Close on Escape (lines 281-288)
    if (key.escape) {
      if (filter) { setFilter(''); return; }
      onClose();
    }

    // Tab to switch to settings (lines 291-294)
    if (key.tab) { onSwitchToSettings(); }

    // Filter mode entry (lines 297-300)
    if (input === '/') { setIsFilterMode(true); }

    // Refresh (lines 303-306)
    if (input === 'r' || input === 'R') { onRefresh(); }

    // Navigation (lines 309-316)
    if (key.upArrow) { navigateUp(); }
    if (key.downArrow) { navigateDown(); }

    // Enter to select/expand (lines 319-331)
    if (key.return) { /* ... */ }

    // Left/Right arrows (lines 334-360)
    if (key.leftArrow) { /* collapse */ }
    if (key.rightArrow) { /* expand */ }
  },
  { isActive: true }
);
```

#### State to REMOVE (Lines 102-115)

```typescript
const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
const [selectedModelIdx, setSelectedModelIdx] = useState(-1);
const [expandedSections, setExpandedSections] = useState<Set<number>>(new Set([0]));
const [filter, setFilter] = useState('');
const [isFilterMode, setIsFilterMode] = useState(false);
const [scrollOffset, setScrollOffset] = useState(0);
```

These are now managed by `useModelSelectorState`.

---

## Implementation Plan

### Step 1: Create ModelSelectorScreen.tsx

**Location**: `src/tui/components/ModelSelectorScreen.tsx`

```tsx
/**
 * ModelSelectorScreen - Orchestrator for model selection
 *
 * TUI-073: Extracts model selector from AgentView.tsx.
 * Composes useModelSelectorState (state) + useInput (keyboard) + ModelSelectorView (UI).
 *
 * Feature: spec/features/model-selector-screen.feature
 */

import React, { useEffect } from 'react';
import { useInput } from 'ink';
import { ModelSelectorView } from './ModelSelectorView';
import { useModelSelectorState } from '../hooks/useModelSelectorState';
import type { ModelSelection } from '../types/provider';

export interface ModelSelectorScreenProps {
  /** Terminal width for layout */
  width: number;
  /** Terminal height for layout */
  height: number;
  /** Currently selected model ID (for highlighting) */
  currentModelId?: string;
  /** Called when a model is selected */
  onSelectModel: (model: ModelSelection) => void;
  /** Called when screen should close */
  onClose: () => void;
  /** Called to switch to provider settings */
  onSwitchToSettings: () => void;
}

export function ModelSelectorScreen({
  width,
  height,
  currentModelId,
  onSelectModel,
  onClose,
  onSwitchToSettings,
}: ModelSelectorScreenProps): React.ReactElement {
  const state = useModelSelectorState();

  // Set visible height based on terminal height
  useEffect(() => {
    state.setVisibleHeight(height - 6); // Account for header/footer
  }, [height, state]);

  // Mark as visible when mounted
  useEffect(() => {
    state.setIsVisible(true);
    return () => state.setIsVisible(false);
  }, [state]);

  // Keyboard handling
  useInput((input, key) => {
    // FILTER MODE
    if (state.isFilterMode) {
      if (key.escape) {
        state.setIsFilterMode(false);
        state.setFilter('');
        return;
      }
      if (key.return) {
        state.setIsFilterMode(false);
        return;
      }
      if (key.backspace || key.delete) {
        state.setFilter(state.filter.slice(0, -1));
        return;
      }
      // Accept printable characters
      const clean = input
        .split('')
        .filter(ch => {
          const code = ch.charCodeAt(0);
          return code >= 32 && code <= 126;
        })
        .join('');
      if (clean) {
        state.setFilter(state.filter + clean);
      }
      return;
    }

    // NORMAL MODE

    // Escape: clear filter or close
    if (key.escape) {
      if (state.filter) {
        state.setFilter('');
        return;
      }
      onClose();
      return;
    }

    // Tab: switch to settings
    if (key.tab) {
      onSwitchToSettings();
      return;
    }

    // Slash: enter filter mode
    if (input === '/') {
      state.setIsFilterMode(true);
      return;
    }

    // Refresh
    if (input === 'r' || input === 'R') {
      void state.refreshModels();
      return;
    }

    // Navigation: Up/Down
    if (key.upArrow) {
      state.navigateUp();
      return;
    }
    if (key.downArrow) {
      state.navigateDown();
      return;
    }

    // Left arrow: collapse section
    if (key.leftArrow) {
      const currentSection = state.providerSections[state.selectedSectionIdx];
      if (currentSection && state.expandedProviders.has(currentSection.providerId)) {
        state.toggleSectionExpansion(currentSection.providerId);
        state.setSelectedModelIdx(-1); // Move to section header
      }
      return;
    }

    // Right arrow: expand section
    if (key.rightArrow) {
      const currentSection = state.providerSections[state.selectedSectionIdx];
      if (currentSection && !state.expandedProviders.has(currentSection.providerId)) {
        state.toggleSectionExpansion(currentSection.providerId);
      }
      return;
    }

    // Enter: select model or toggle section
    if (key.return) {
      const flatIdx = state.getCurrentFlatIndex();
      const item = state.filteredFlatItems[flatIdx];
      
      if (!item) return;
      
      if (item.type === 'section') {
        state.toggleSectionExpansion(item.section.providerId);
      } else if (item.type === 'model') {
        const selection = state.selectModel(item.section, item.model);
        onSelectModel(selection);
        onClose();
      }
      return;
    }
  }, { isActive: true });

  // Render presentation component
  return (
    <ModelSelectorView
      width={width}
      height={height}
      sections={state.providerSections}
      flatItems={state.filteredFlatItems}
      selectedSectionIdx={state.selectedSectionIdx}
      selectedModelIdx={state.selectedModelIdx}
      expandedProviders={state.expandedProviders}
      scrollOffset={state.scrollOffset}
      visibleHeight={state.visibleHeight}
      filter={state.filter}
      isFilterMode={state.isFilterMode}
      currentModelId={currentModelId}
      isRefreshing={state.isRefreshing}
    />
  );
}
```

### Step 2: Modify ModelSelectorView.tsx

1. **Remove** `useInput` handler (lines 250-363)
2. **Remove** internal state (lines 102-115)
3. **Add** new props to receive state from parent:

```tsx
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

4. **Remove** navigation callbacks from props (`onClose`, `onSwitchToSettings`, `onSelectModel`, `onRefresh`)

### Step 3: Update AgentView.tsx

1. **Keep**: `showModelSelector`, `setShowModelSelector`
2. **Delete**: All other model selector state variables
3. **Delete**: Model selector keyboard handling block
4. **Delete**: Inline rendering block
5. **Add**: ModelSelectorScreen component:

```tsx
import { ModelSelectorScreen } from './ModelSelectorScreen';

// In render:
if (showModelSelector) {
  return (
    <ModelSelectorScreen
      width={terminalWidth}
      height={terminalHeight}
      currentModelId={currentModel?.apiModelId}
      onSelectModel={(model) => {
        setCurrentModel(model);
        setShowModelSelector(false);
      }}
      onClose={() => setShowModelSelector(false)}
      onSwitchToSettings={() => {
        setShowModelSelector(false);
        setShowSettingsTab(true);
        void loadProviderStatuses();
      }}
    />
  );
}
```

---

## useModelSelectorState Hook API Reference

From `src/tui/hooks/useModelSelectorState.ts`:

```typescript
interface UseModelSelectorStateReturn {
  // Data
  currentModel: ModelSelection | null;
  providerSections: ProviderSection[];
  flatItems: ModelSelectorItem[];
  filteredFlatItems: ModelSelectorItem[];
  isLoading: boolean;
  isRefreshing: boolean;
  modelsInitialized: boolean;

  // Selection state
  selectedSectionIdx: number;
  selectedModelIdx: number;
  expandedProviders: Set<string>;

  // Scroll/filter state
  scrollOffset: number;
  visibleHeight: number;
  filter: string;
  isFilterMode: boolean;

  // Visibility
  isVisible: boolean;

  // Actions
  setCurrentModel: (model: ModelSelection | null) => void;
  setSelectedSectionIdx: (idx: number) => void;
  setSelectedModelIdx: (idx: number) => void;
  setScrollOffset: (offset: number) => void;
  setVisibleHeight: (height: number) => void;
  setFilter: (filter: string) => void;
  setIsFilterMode: (mode: boolean) => void;
  setIsVisible: (visible: boolean) => void;

  // Operations
  toggleSectionExpansion: (providerId: string) => void;
  refreshModels: () => Promise<void>;
  loadModels: () => Promise<void>;
  selectModel: (section: ProviderSection, model: NapiModelInfo) => ModelSelection;

  // Navigation helpers
  navigateUp: () => void;
  navigateDown: () => void;
  getCurrentFlatIndex: () => number;
}
```

---

## Type Definitions

From `src/tui/types/provider.ts`:

```typescript
interface ProviderSection {
  providerId: string;
  providerName: string;
  internalName: string;
  models: ProviderModel[];
  hasCredentials: boolean;
  profileName?: string;
  profileConfig?: ProfileConfig;
  isUnreachable?: boolean;
}

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

type ModelSelectorItem =
  | { type: 'section'; sectionIdx: number; section: ProviderSection; isExpanded: boolean; }
  | { type: 'model'; sectionIdx: number; modelIdx: number; section: ProviderSection; model: NapiModelInfo; };
```

---

## Keyboard Mapping

| Key | Normal Mode | Filter Mode |
|-----|-------------|-------------|
| `↑` | Navigate up | - |
| `↓` | Navigate down | - |
| `←` | Collapse section | - |
| `→` | Expand section | - |
| `Enter` | Select model / Toggle section | Exit filter mode |
| `Escape` | Clear filter or Close | Clear filter & exit mode |
| `Tab` | Switch to settings | - |
| `/` | Enter filter mode | - |
| `r`/`R` | Refresh models | - |
| Printable | - | Append to filter |
| `Backspace` | - | Remove last char |

---

## Testing Scenarios (17 total)

### Navigation (4)
- Navigate down in model list
- Navigate up in model list
- Collapse section with Left arrow
- Expand section with Right arrow

### Close Behavior (2)
- Close screen with Escape when no filter active
- Clear filter with Escape when filter active

### Screen Switching (1)
- Switch to provider settings with Tab

### Model Selection (2)
- Select a model with Enter
- Toggle section expansion with Enter on section header

### Filter Mode (5)
- Enter filter mode with slash key
- Type characters in filter mode
- Delete characters with backspace
- Exit filter mode with Enter
- Clear filter and exit with Escape

### Utility (1)
- Refresh models with r key

### Component Structure (2)
- ModelSelectorScreen uses useModelSelectorState hook
- ModelSelectorView is purely presentational

---

## Expected Line Count Changes

| File | Before | After | Change |
|------|--------|-------|--------|
| AgentView.tsx | ~8400 | ~8050 | -350 lines |
| ModelSelectorView.tsx | 525 | ~400 | -125 lines |
| ModelSelectorScreen.tsx | 0 | ~150 | +150 lines |
| **Net** | | | **-325 lines** |

Plus improved separation of concerns and testability.
