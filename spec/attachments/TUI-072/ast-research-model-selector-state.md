# AST Research: Model Selector State in AgentView.tsx

## Research Date
2026-02-24

## Purpose
Identify all model selector related state, helper functions, and logic to extract into useModelSelectorState hook.

## AST Search Results

### Model Selector State Declarations (Lines 1049-1082)

Pattern: `const [$NAME, $SETNAME] = useState($INIT)`

Found model selector related useState calls:
- Line 1049: `const [modelsInitialized, setModelsInitialized] = useState(false)`
- Line 1050: `const [showModelSelector, setShowModelSelector] = useState(false)`
- Line 1051: `const [selectedSectionIdx, setSelectedSectionIdx] = useState(0)`
- Line 1052: `const [selectedModelIdx, setSelectedModelIdx] = useState(-1)` // -1 = on section header
- Line 1056: `const [modelSelectorScrollOffset, setModelSelectorScrollOffset] = useState(0)`
- Line 1057: `const [modelSelectorFilter, setModelSelectorFilter] = useState('')`
- Line 1058: `const [isModelSelectorFilterMode, setIsModelSelectorFilterMode] = useState(false)`
- Line 1082: `const [isRefreshingModels, setIsRefreshingModels] = useState(false)`

Additional state used by model selector:
- `currentModel` - ModelSelection type
- `providerSections` - ProviderSection[]
- `expandedProviders` - Set<string>

### Helper Functions (Lines 256-344)

Found with pattern `const buildFlatModelList = $BODY`:
- Line 256: `buildFlatModelList(sections, expandedProviders)` - Builds flattened list from sections and expanded state
- Line 274: `flatIndexToSectionModel(flatIndex, items)` - Convert flat index to (sectionIdx, modelIdx)
- Line 287: `sectionModelToFlatIndex(sectionIdx, modelIdx, items)` - Convert (sectionIdx, modelIdx) to flat index
- Line 313: `mapProviderIdToInternal(providerId)` - Maps models.dev IDs to internal (anthropic→claude, google→gemini)
- Line 324: `mapInternalToProviderId(internalName)` - Reverse mapping (claude→anthropic, gemini→google)
- Line 337: `mapModelsDevToRegistryId(modelsDevProviderId)` - Maps models.dev to registry/credentials IDs

### Model Loading Logic (Lines 1873-2010)

Model loading occurs in `handleInitializeApp`:
1. Call `modelsListAll()` to fetch all provider models from models.dev
2. Filter by providers with credentials using `getProviderConfig()`
3. Load profile sections using `loadProviderProfiles()` for SUPPORTED_PROVIDERS
4. For each profile, call `modelsListLocalOpenai(profile.baseUrl)` to fetch models
5. Combine cloud and profile sections: `[...profileSections, ...cloudSections]`
6. Set state with `setProviderSections(sections)`

### useMemo for flatItems (Line 1548)

```typescript
const flatItems = useMemo(
  () => buildFlatModelList(providerSections, expandedProviders),
  [providerSections, expandedProviders]
);
```

### Navigation Logic (approx. Lines 6134-6150)

Found in key handlers:
- `sectionModelToFlatIndex(selectedSectionIdx, selectedModelIdx, filteredFlatItems)` - Get current flat index
- `flatIndexToSectionModel(newFlatIdx, filteredFlatItems)` - Convert back to section/model indices
- Auto-scroll logic updates `modelSelectorScrollOffset`

## Extraction Plan

1. **Types** - Import from `src/tui/types/provider.ts`:
   - `ProviderSection`
   - `ModelSelectorItem`
   - `ModelSelection`
   - `ProviderModel` (NapiModelInfo alias)

2. **Helper Functions** - Move to hook module:
   - `buildFlatModelList` - Pure function, returns ModelSelectorItem[]
   - `flatIndexToSectionModel` - Pure function
   - `sectionModelToFlatIndex` - Pure function
   - `mapProviderIdToInternal` - Pure function
   - `mapInternalToProviderId` - Pure function
   - `mapModelsDevToRegistryId` - Pure function

3. **Hook State** - useState calls:
   - `isLoading` (initial true, false after load)
   - `isRefreshing` (for refresh operation)
   - `modelsInitialized` (tracks initial load completion)
   - `isVisible` / `showModelSelector` (visibility control)
   - `providerSections` - ProviderSection[]
   - `currentModel` - ModelSelection | null
   - `selectedSectionIdx` - number
   - `selectedModelIdx` - number (-1 for section header)
   - `expandedProviders` - Set<string>
   - `scrollOffset` - number
   - `filter` - string
   - `isFilterMode` - boolean

4. **Computed Values** - useMemo:
   - `flatItems` - from buildFlatModelList(providerSections, expandedProviders)
   - `filteredFlatItems` - flatItems filtered by filter string

5. **Operations** - useCallback:
   - `loadModels()` - Load from NAPI on mount
   - `refreshModels()` - Call modelsRefreshCache() then reload
   - `toggleSectionExpansion(providerId)` - Toggle expanded state
   - `navigateUp()` - Move selection up with auto-scroll
   - `navigateDown()` - Move selection down with auto-scroll
   - `getCurrentFlatIndex()` - Get current position in flat list
   - `selectModel(section, model)` - Build ModelSelection from section/model

## Dependencies

NAPI Functions:
- `modelsListAll` - Returns NapiProviderModels[]
- `modelsListLocalOpenai` - Returns model IDs for local OpenAI-compatible server
- `modelsRefreshCache` - Refreshes model cache

Provider Config:
- `loadProviderProfiles(providerId)` - Load profiles for a provider
- `getProviderConfig(providerId)` - Get credentials/config
- `getProviderRegistryEntry(providerId)` - Get registry metadata

## Test Strategy

1. Unit test helper functions (pure functions, easy to test)
2. Mock NAPI functions for hook tests
3. Test loading state transitions
4. Test navigation helpers with various expansion states
5. Test filter functionality
6. Test scroll offset auto-adjustment
