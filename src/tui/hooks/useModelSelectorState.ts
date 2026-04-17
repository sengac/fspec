/**
 * useModelSelectorState - Hook for model selector state management
 *
 * TUI-072: Extracts model selector state from AgentView.tsx into a dedicated hook.
 * TUI-075: Uses shared Zustand store for model data (providerSections, currentModel, etc.)
 * MODEL-004: Composes useCustomModelFormState for custom model CRUD.
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
import {
  lookupFacadeOverride,
  lookupCompactionThreshold,
} from '../utils/custom-model-utils';
import {
  buildFlatModelList,
  flatIndexToSectionModel,
  sectionModelToFlatIndex,
  filterFlatItems,
} from '../utils/flat-model-list';
import {
  useCustomModelFormState,
  type UseCustomModelFormStateReturn,
} from './useCustomModelFormState';

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

// Re-export flat list utilities for backwards compatibility
export {
  buildFlatModelList,
  flatIndexToSectionModel,
  sectionModelToFlatIndex,
} from '../utils/flat-model-list';

// =============================================================================
// HOOK INTERFACE
// =============================================================================

export interface UseModelSelectorStateReturn
  extends UseCustomModelFormStateReturn {
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

  const currentModel = useCurrentModel();
  const providerSections = useProviderSections();
  const modelsInitialized = useModelsInitialized();
  const isLoading = useIsModelsLoading();
  const isRefreshing = useIsModelsRefreshing();

  const store = useModelStore.getState();

  // -------------------------------------------------------------------------
  // LOCAL UI STATE (selection, scroll, filter - not shared)
  // -------------------------------------------------------------------------

  const [selectedSectionIdx, setSelectedSectionIdx] = useState(0);
  const [selectedModelIdx, setSelectedModelIdx] = useState(-1);
  const [expandedProviders, setExpandedProviders] = useState<Set<string>>(
    new Set()
  );
  const [scrollOffset, setScrollOffset] = useState(0);
  const [visibleHeight, setVisibleHeight] = useState(10);
  const [filter, setFilter] = useState('');
  const [isFilterMode, setIsFilterMode] = useState(false);
  const [isVisible, setIsVisible] = useState(false);

  const prevIsVisible = useRef(isVisible);

  // -------------------------------------------------------------------------
  // MODEL-004: CUSTOM MODEL FORM STATE (composed hook)
  // -------------------------------------------------------------------------

  const customModelFormState = useCustomModelFormState();

  // -------------------------------------------------------------------------
  // COMPUTED VALUES (delegates to flat-model-list utilities)
  // -------------------------------------------------------------------------

  const flatItems = useMemo(
    () => buildFlatModelList(providerSections, expandedProviders),
    [providerSections, expandedProviders]
  );

  const filteredFlatItems = useMemo(
    () => filterFlatItems(flatItems, filter),
    [flatItems, filter]
  );

  // -------------------------------------------------------------------------
  // OPERATIONS
  // -------------------------------------------------------------------------

  const loadModels = useCallback(async () => {
    await initializeModels();
  }, []);

  const refreshModels = useCallback(async () => {
    store.setIsRefreshing(true);
    try {
      await modelsRefreshCache();
      store.setModelsInitialized(false);
      await loadModels();
    } finally {
      store.setIsRefreshing(false);
    }
  }, [loadModels, store]);

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

  const getCurrentFlatIndex = useCallback(() => {
    return sectionModelToFlatIndex(
      selectedSectionIdx,
      selectedModelIdx,
      filteredFlatItems
    );
  }, [selectedSectionIdx, selectedModelIdx, filteredFlatItems]);

  const navigateDown = useCallback(() => {
    const currentIdx = getCurrentFlatIndex();
    const newIdx = Math.min(currentIdx + 1, filteredFlatItems.length - 1);
    const { sectionIdx, modelIdx } = flatIndexToSectionModel(
      newIdx,
      filteredFlatItems
    );
    setSelectedSectionIdx(sectionIdx);
    setSelectedModelIdx(modelIdx);

    if (newIdx >= scrollOffset + visibleHeight) {
      setScrollOffset(newIdx - visibleHeight + 1);
    }
  }, [getCurrentFlatIndex, filteredFlatItems, scrollOffset, visibleHeight]);

  const navigateUp = useCallback(() => {
    const currentIdx = getCurrentFlatIndex();
    const newIdx = Math.max(currentIdx - 1, 0);
    const { sectionIdx, modelIdx } = flatIndexToSectionModel(
      newIdx,
      filteredFlatItems
    );
    setSelectedSectionIdx(sectionIdx);
    setSelectedModelIdx(modelIdx);

    if (newIdx < scrollOffset) {
      setScrollOffset(newIdx);
    }
  }, [getCurrentFlatIndex, filteredFlatItems, scrollOffset]);

  /**
   * Build ModelSelection from section and model.
   * MODEL-004: Also looks up facade from custom model config.
   * CTX-008: Also looks up compaction threshold from custom model / profile config.
   */
  const selectModel = useCallback(
    (section: ProviderSection, model: NapiModelInfo): ModelSelection => {
      const facade = lookupFacadeOverride(section, model.id);
      const compactionThreshold = lookupCompactionThreshold(section, model.id);

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
        facade,
        compactionThreshold,
      };
    },
    []
  );

  // -------------------------------------------------------------------------
  // EFFECTS
  // -------------------------------------------------------------------------

  useEffect(() => {
    if (!modelsInitialized && !isLoading) {
      void loadModels();
    }
  }, [loadModels, modelsInitialized, isLoading]);

  useEffect(() => {
    if (isVisible && !prevIsVisible.current) {
      setScrollOffset(0);
      setFilter('');
      setIsFilterMode(false);
    }
    prevIsVisible.current = isVisible;
  }, [isVisible]);

  useEffect(() => {
    if (filter && filteredFlatItems.length > 0) {
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
    currentModel,
    providerSections,
    flatItems,
    filteredFlatItems,
    isLoading,
    isRefreshing,
    modelsInitialized,
    selectedSectionIdx,
    selectedModelIdx,
    expandedProviders,
    scrollOffset,
    visibleHeight,
    filter,
    isFilterMode,
    isVisible,
    // MODEL-004: Custom model form state (spread from composed hook)
    ...customModelFormState,
    setCurrentModel: store.setCurrentModel,
    setSelectedSectionIdx,
    setSelectedModelIdx,
    setScrollOffset,
    setVisibleHeight,
    setFilter,
    setIsFilterMode,
    setIsVisible,
    toggleSectionExpansion,
    refreshModels,
    loadModels,
    selectModel,
    navigateUp,
    navigateDown,
    getCurrentFlatIndex,
  };
}
