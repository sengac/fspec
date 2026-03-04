/**
 * useModelSelectorState - Hook for model selector state management
 *
 * TUI-072: Extracts model selector state from AgentView.tsx into a dedicated hook.
 * TUI-075: Uses shared Zustand store for model data (providerSections, currentModel, etc.)
 *
 * UI-only state (selection, scroll, filter) remains local to the hook.
 * Shared data (models, loading state) comes from the modelStore.
 *
 * Feature: spec/features/model-selector-state-hook.feature
 */

import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { modelsRefreshCache } from '@sengac/codelet-napi';
import type { NapiModelInfo } from '@sengac/codelet-napi';
import type { ModelSelectorItem } from '../types/provider';
import {
  useModelStore,
  useProviderSections,
  useCurrentModel,
  useModelsInitialized,
  useIsModelsLoading,
  useIsModelsRefreshing,
  type ProviderSection,
  type ModelSelection,
} from '../store/modelStore';
import {
  initializeModels,
  extractModelIdForRegistry,
} from '../services/modelInitializationService';

// PROV-008: Import provider mapping from shared utility (DRY)
import {
  mapProviderIdToInternal,
  mapInternalToProviderId,
  mapModelsDevToRegistryId,
} from '../utils/provider-mapping';

// Re-export for backwards compatibility with existing consumers
export {
  mapProviderIdToInternal,
  mapInternalToProviderId,
  mapModelsDevToRegistryId,
};

// Re-export from service — DRY: single authoritative implementation (FIX-3)
export { extractModelIdForRegistry } from '../services/modelInitializationService';

// =============================================================================
// HELPER FUNCTIONS (Pure functions for flat list operations)
// =============================================================================

/**
 * Build flattened list from sections and expanded state
 */
export const buildFlatModelList = (
  sections: ProviderSection[],
  expandedProviders: Set<string>
): ModelSelectorItem[] => {
  const items: ModelSelectorItem[] = [];
  sections.forEach((section, sectionIdx) => {
    const isExpanded = expandedProviders.has(section.providerId);
    items.push({ type: 'section', sectionIdx, section, isExpanded });
    if (isExpanded) {
      section.models.forEach((model, modelIdx) => {
        items.push({ type: 'model', sectionIdx, modelIdx, section, model });
      });
    }
  });
  return items;
};

/**
 * Convert flat index to (sectionIdx, modelIdx)
 * modelIdx is -1 for section headers
 */
export const flatIndexToSectionModel = (
  flatIndex: number,
  items: ModelSelectorItem[]
): { sectionIdx: number; modelIdx: number } => {
  const item = items[flatIndex];
  if (!item) {
    return { sectionIdx: 0, modelIdx: -1 };
  }
  if (item.type === 'section') {
    return { sectionIdx: item.sectionIdx, modelIdx: -1 };
  }
  return { sectionIdx: item.sectionIdx, modelIdx: item.modelIdx };
};

/**
 * Convert (sectionIdx, modelIdx) to flat index
 */
export const sectionModelToFlatIndex = (
  sectionIdx: number,
  modelIdx: number,
  items: ModelSelectorItem[]
): number => {
  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    if (
      item.type === 'section' &&
      item.sectionIdx === sectionIdx &&
      modelIdx === -1
    ) {
      return i;
    }
    if (
      item.type === 'model' &&
      item.sectionIdx === sectionIdx &&
      item.modelIdx === modelIdx
    ) {
      return i;
    }
  }
  return 0;
};

// =============================================================================
// HOOK INTERFACE
// =============================================================================

export interface UseModelSelectorStateReturn {
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
  selectModel: (
    section: ProviderSection,
    model: NapiModelInfo
  ) => ModelSelection;

  // Navigation helpers
  navigateUp: () => void;
  navigateDown: () => void;
  getCurrentFlatIndex: () => number;
}

// =============================================================================
// HOOK IMPLEMENTATION
// =============================================================================

export function useModelSelectorState(): UseModelSelectorStateReturn {
  // -------------------------------------------------------------------------
  // SHARED STATE (from Zustand store)
  // -------------------------------------------------------------------------

  // TUI-075: Model data comes from shared store
  const currentModel = useCurrentModel();
  const providerSections = useProviderSections();
  const modelsInitialized = useModelsInitialized();
  const isLoading = useIsModelsLoading();
  const isRefreshing = useIsModelsRefreshing();

  // Store actions for updating shared state
  const store = useModelStore.getState();

  // -------------------------------------------------------------------------
  // LOCAL UI STATE (selection, scroll, filter - not shared)
  // -------------------------------------------------------------------------

  // Selection state
  const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
  const [selectedModelIdx, setSelectedModelIdx] = useState(-1); // -1 = section header
  const [expandedProviders, setExpandedProviders] = useState<Set<string>>(
    new Set()
  );

  // Scroll/filter state
  const [scrollOffset, setScrollOffset] = useState(0);
  const [visibleHeight, setVisibleHeight] = useState(10);
  const [filter, setFilter] = useState('');
  const [isFilterMode, setIsFilterMode] = useState(false);

  // Visibility state
  const [isVisible, setIsVisible] = useState(false);

  // Track previous visibility for reset on open
  const prevIsVisible = useRef(isVisible);

  // -------------------------------------------------------------------------
  // COMPUTED VALUES
  // -------------------------------------------------------------------------

  /**
   * Build flat list from sections and expanded providers
   */
  const flatItems = useMemo(
    () => buildFlatModelList(providerSections, expandedProviders),
    [providerSections, expandedProviders]
  );

  /**
   * Filter flat items by filter string (case-insensitive)
   * Matches provider name, model ID, or model name
   */
  const filteredFlatItems = useMemo(() => {
    if (!filter) {
      return flatItems;
    }

    const filterLower = filter.toLowerCase();
    const matchingSectionIdxs = new Set<number>();

    // First pass: find matching sections and models
    flatItems.forEach(item => {
      if (item.type === 'section') {
        // Check provider name and ID
        if (
          item.section.providerName.toLowerCase().includes(filterLower) ||
          item.section.providerId.toLowerCase().includes(filterLower)
        ) {
          matchingSectionIdxs.add(item.sectionIdx);
        }
      } else if (item.type === 'model') {
        // Check model ID and name
        if (
          item.model.id.toLowerCase().includes(filterLower) ||
          item.model.name.toLowerCase().includes(filterLower)
        ) {
          matchingSectionIdxs.add(item.sectionIdx);
        }
      }
    });

    // Second pass: build filtered list
    return flatItems.filter(item => {
      if (item.type === 'section') {
        return matchingSectionIdxs.has(item.sectionIdx);
      }
      // For models, check if model itself matches
      if (item.type === 'model') {
        const modelMatches =
          item.model.id.toLowerCase().includes(filterLower) ||
          item.model.name.toLowerCase().includes(filterLower);
        const sectionMatches =
          item.section.providerName.toLowerCase().includes(filterLower) ||
          item.section.providerId.toLowerCase().includes(filterLower);
        return modelMatches || sectionMatches;
      }
      return false;
    });
  }, [flatItems, filter]);

  // -------------------------------------------------------------------------
  // OPERATIONS
  // -------------------------------------------------------------------------

  /**
   * Load models from NAPI (both cloud and local profiles)
   * TUI-075: Uses shared initializeModels function from modelInitializationService
   */
  const loadModels = useCallback(async () => {
    await initializeModels();
  }, []);

  /**
   * Refresh models (clear cache and reload)
   * TUI-075: Updates shared store instead of local state
   */
  const refreshModels = useCallback(async () => {
    store.setIsRefreshing(true);
    try {
      await modelsRefreshCache();
      // Reset initialized flag so initializeModels re-fetches
      store.setModelsInitialized(false);
      await loadModels();
    } finally {
      store.setIsRefreshing(false);
    }
  }, [loadModels, store]);

  /**
   * Toggle section expansion
   */
  const toggleSectionExpansion = useCallback((providerId: string) => {
    setExpandedProviders(prev => {
      const next = new Set(prev);
      if (next.has(providerId)) {
        next.delete(providerId);
      } else {
        next.add(providerId);
      }
      return next;
    });
  }, []);

  /**
   * Get current flat index from section/model indices
   */
  const getCurrentFlatIndex = useCallback(() => {
    return sectionModelToFlatIndex(
      selectedSectionIdx,
      selectedModelIdx,
      filteredFlatItems
    );
  }, [selectedSectionIdx, selectedModelIdx, filteredFlatItems]);

  /**
   * Navigate down in the list
   */
  const navigateDown = useCallback(() => {
    const currentIdx = getCurrentFlatIndex();
    const newIdx = Math.min(currentIdx + 1, filteredFlatItems.length - 1);
    const { sectionIdx, modelIdx } = flatIndexToSectionModel(
      newIdx,
      filteredFlatItems
    );
    setSelectedSectionIdx(sectionIdx);
    setSelectedModelIdx(modelIdx);

    // Auto-scroll: keep selection visible (Rule [12])
    if (newIdx >= scrollOffset + visibleHeight) {
      setScrollOffset(newIdx - visibleHeight + 1);
    }
  }, [getCurrentFlatIndex, filteredFlatItems, scrollOffset, visibleHeight]);

  /**
   * Navigate up in the list
   */
  const navigateUp = useCallback(() => {
    const currentIdx = getCurrentFlatIndex();
    const newIdx = Math.max(currentIdx - 1, 0);
    const { sectionIdx, modelIdx } = flatIndexToSectionModel(
      newIdx,
      filteredFlatItems
    );
    setSelectedSectionIdx(sectionIdx);
    setSelectedModelIdx(modelIdx);

    // Auto-scroll: keep selection visible
    if (newIdx < scrollOffset) {
      setScrollOffset(newIdx);
    }
  }, [getCurrentFlatIndex, filteredFlatItems, scrollOffset]);

  /**
   * Build ModelSelection from section and model
   */
  const selectModel = useCallback(
    (section: ProviderSection, model: NapiModelInfo): ModelSelection => {
      return {
        providerId: section.providerId,
        modelId: extractModelIdForRegistry(model.id),
        apiModelId: model.id,
        displayName: model.name,
        reasoning: model.reasoning,
        hasVision: model.hasVision,
        contextWindow: model.contextWindow,
        maxOutput: model.maxOutput,
        profileName: section.profileName,
        profileConfig: section.profileConfig,
      };
    },
    []
  );

  // -------------------------------------------------------------------------
  // EFFECTS
  // -------------------------------------------------------------------------

  // Load models on mount (only if not already initialized)
  // TUI-075: Models are shared via store, so only load once
  useEffect(() => {
    if (!modelsInitialized && !isLoading) {
      void loadModels();
    }
  }, [loadModels, modelsInitialized, isLoading]);

  // Reset scroll/filter when model selector opens
  useEffect(() => {
    if (isVisible && !prevIsVisible.current) {
      // Selector just opened
      setScrollOffset(0);
      setFilter('');
      setIsFilterMode(false);
    }
    prevIsVisible.current = isVisible;
  }, [isVisible]);

  // Reset selection when filter changes
  useEffect(() => {
    if (filter && filteredFlatItems.length > 0) {
      // Move selection to first item
      const { sectionIdx, modelIdx } = flatIndexToSectionModel(
        0,
        filteredFlatItems
      );
      setSelectedSectionIdx(sectionIdx);
      setSelectedModelIdx(modelIdx);
      setScrollOffset(0);
    }
  }, [filter, filteredFlatItems]);

  // -------------------------------------------------------------------------
  // RETURN
  // -------------------------------------------------------------------------

  return {
    // Data (from shared store)
    currentModel,
    providerSections,
    flatItems,
    filteredFlatItems,
    isLoading,
    isRefreshing,
    modelsInitialized,

    // Selection state (local)
    selectedSectionIdx,
    selectedModelIdx,
    expandedProviders,

    // Scroll/filter state (local)
    scrollOffset,
    visibleHeight,
    filter,
    isFilterMode,

    // Visibility (local)
    isVisible,

    // Actions - TUI-075: setCurrentModel uses shared store
    setCurrentModel: store.setCurrentModel,
    setSelectedSectionIdx,
    setSelectedModelIdx,
    setScrollOffset,
    setVisibleHeight,
    setFilter,
    setIsFilterMode,
    setIsVisible,

    // Operations
    toggleSectionExpansion,
    refreshModels,
    loadModels,
    selectModel,

    // Navigation helpers
    navigateUp,
    navigateDown,
    getCurrentFlatIndex,
  };
}
