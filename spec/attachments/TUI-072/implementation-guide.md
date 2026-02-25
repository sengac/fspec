# TUI-072: Create useModelSelectorState Hook

## Overview

Extract model selector state management from `AgentView.tsx` into a dedicated hook following the pattern of `useProviderSettingsState.ts`.

## Source Code to Extract

### From `src/tui/components/AgentView.tsx`

#### 1. State Declarations (Lines ~1090-1128)

```typescript
// TUI-034: Model selection state
const [currentModel, setCurrentModel] = useState<ModelSelection | null>(null);
const [providerSections, setProviderSections] = useState<ProviderSection[]>([]);
const [modelsInitialized, setModelsInitialized] = useState(false);
const [showModelSelector, setShowModelSelector] = useState(false);
const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
const [selectedModelIdx, setSelectedModelIdx] = useState(-1); // -1 = on section header
const [expandedProviders, setExpandedProviders] = useState<Set<string>>(new Set());
const [modelSelectorScrollOffset, setModelSelectorScrollOffset] = useState(0);
const [modelSelectorFilter, setModelSelectorFilter] = useState('');
const [isModelSelectorFilterMode, setIsModelSelectorFilterMode] = useState(false);
const [isRefreshingModels, setIsRefreshingModels] = useState(false);
```

#### 2. Helper Functions (Lines ~302-391)

```typescript
// Build flattened list from sections and expanded state
const buildFlatModelList = (
  sections: ProviderSection[],
  expandedProviders: Set<string>
): ModelSelectorItem[] => { ... }

// Convert flat index to (sectionIdx, modelIdx)
const flatIndexToSectionModel = (
  flatIndex: number,
  items: ModelSelectorItem[]
): { sectionIdx: number; modelIdx: number } => { ... }

// Convert (sectionIdx, modelIdx) to flat index
const sectionModelToFlatIndex = (
  sectionIdx: number,
  modelIdx: number,
  items: ModelSelectorItem[]
): number => { ... }

// Provider ID mapping functions
const mapProviderIdToInternal = (providerId: string): string => { ... }
const mapInternalToProviderId = (internalName: string): string => { ... }
const mapModelsDevToRegistryId = (modelsDevProviderId: string): string => { ... }
```

#### 3. Model Loading Logic

Search for `loadProviderSections` or similar model loading code. Key functions to extract:
- Loading models from `modelsListAll()`
- Loading local OpenAI models from `modelsListLocalOpenai()`
- Refreshing model cache with `modelsRefreshCache()`

## Target File

Create: `src/tui/hooks/useModelSelectorState.ts`

## Hook Interface

```typescript
export interface UseModelSelectorStateReturn {
  // Data
  currentModel: ModelSelection | null;
  providerSections: ProviderSection[];
  flatItems: ModelSelectorItem[];
  isLoading: boolean;
  isRefreshing: boolean;

  // Selection state
  selectedSectionIdx: number;
  selectedModelIdx: number;
  expandedProviders: Set<string>;

  // Scroll/filter state
  scrollOffset: number;
  filter: string;
  isFilterMode: boolean;

  // Visibility
  isVisible: boolean;

  // Actions
  setCurrentModel: (model: ModelSelection | null) => void;
  setSelectedSectionIdx: (idx: number) => void;
  setSelectedModelIdx: (idx: number) => void;
  setScrollOffset: (offset: number) => void;
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

## Reference Implementation

Follow the pattern in `src/tui/hooks/useProviderSettingsState.ts`:
- Use `useState` for all state
- Use `useCallback` for all operations
- Use `useMemo` for computed values like `flatItems`
- Load data on mount with `useEffect`

## Dependencies

- Types from `src/tui/types/provider.ts` (will be consolidated in TUI-076)
- NAPI functions: `modelsListAll`, `modelsListLocalOpenai`, `modelsRefreshCache`
- Provider config utilities from `src/utils/provider-config.ts`

## Testing Considerations

- Test model loading with mock NAPI responses
- Test flat list building with various expansion states
- Test navigation helpers (up/down movement)
- Test filter functionality
