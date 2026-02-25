/**
 * modelStore.ts - Zustand Store for Model Selection State
 *
 * TUI-075: Shared model state store for AgentView and ModelSelectorScreen.
 *
 * Key Responsibilities:
 * 1. Store provider sections (loaded models from models.dev + local profiles)
 * 2. Store current model selection
 * 3. Track initialization and loading state
 * 4. Provide selector hooks for optimal re-render performance
 *
 * IMPORTANT: This store is the single source of truth for model data.
 * useModelSelectorState hook and AgentView both read from this store.
 */

import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { useShallow } from 'zustand/react/shallow';
import { logger } from '../../utils/logger';
import type { ProviderSection, ModelSelection } from '../types/provider';

// Re-export types for consumers
export type { ProviderSection, ModelSelection };

// =============================================================================
// STORE INTERFACE
// =============================================================================

interface ModelStoreState {
  // ===== Model Data State =====

  /**
   * List of provider sections with their models.
   * Populated by useModelSelectorState.loadModels().
   */
  providerSections: ProviderSection[];

  /**
   * Currently selected model.
   * null means no model is selected yet.
   */
  currentModel: ModelSelection | null;

  /**
   * Whether models have been initialized (loaded at least once).
   */
  modelsInitialized: boolean;

  /**
   * Whether models are currently being loaded.
   */
  isLoading: boolean;

  /**
   * Whether models are being refreshed (cache invalidation + reload).
   */
  isRefreshing: boolean;

  // ===== Actions =====

  /**
   * Set provider sections (called after loading models).
   */
  setProviderSections: (sections: ProviderSection[]) => void;

  /**
   * Set current model selection.
   */
  setCurrentModel: (model: ModelSelection | null) => void;

  /**
   * Set models initialized flag.
   */
  setModelsInitialized: (initialized: boolean) => void;

  /**
   * Set loading state.
   */
  setIsLoading: (loading: boolean) => void;

  /**
   * Set refreshing state.
   */
  setIsRefreshing: (refreshing: boolean) => void;

  /**
   * Reset store to initial state.
   */
  reset: () => void;
}

// =============================================================================
// STORE IMPLEMENTATION
// =============================================================================

export const useModelStore = create<ModelStoreState>()(
  immer(set => ({
    // Initial state
    providerSections: [],
    currentModel: null,
    modelsInitialized: false,
    isLoading: false,
    isRefreshing: false,

    // Actions
    setProviderSections: (sections: ProviderSection[]) => {
      logger.debug(
        `[ModelStore] setProviderSections: ${sections.length} sections`
      );
      set(state => {
        state.providerSections = sections;
      });
    },

    setCurrentModel: (model: ModelSelection | null) => {
      logger.debug(
        `[ModelStore] setCurrentModel: ${model ? `${model.providerId}/${model.modelId}` : 'null'}`
      );
      set(state => {
        state.currentModel = model;
      });
    },

    setModelsInitialized: (initialized: boolean) => {
      logger.debug(`[ModelStore] setModelsInitialized: ${initialized}`);
      set(state => {
        state.modelsInitialized = initialized;
      });
    },

    setIsLoading: (loading: boolean) => {
      logger.debug(`[ModelStore] setIsLoading: ${loading}`);
      set(state => {
        state.isLoading = loading;
      });
    },

    setIsRefreshing: (refreshing: boolean) => {
      logger.debug(`[ModelStore] setIsRefreshing: ${refreshing}`);
      set(state => {
        state.isRefreshing = refreshing;
      });
    },

    reset: () => {
      logger.debug('[ModelStore] reset');
      set(state => {
        state.providerSections = [];
        state.currentModel = null;
        state.modelsInitialized = false;
        state.isLoading = false;
        state.isRefreshing = false;
      });
    },
  }))
);

// =============================================================================
// SELECTOR HOOKS (avoids re-renders from unused state)
// =============================================================================

export const useProviderSections = () =>
  useModelStore(state => state.providerSections);

export const useCurrentModel = () => useModelStore(state => state.currentModel);

export const useModelsInitialized = () =>
  useModelStore(state => state.modelsInitialized);

export const useIsModelsLoading = () => useModelStore(state => state.isLoading);

export const useIsModelsRefreshing = () =>
  useModelStore(state => state.isRefreshing);

// =============================================================================
// ACTION HOOKS (stable references with shallow comparison)
// =============================================================================

export const useModelStoreActions = () =>
  useModelStore(
    useShallow(state => ({
      setProviderSections: state.setProviderSections,
      setCurrentModel: state.setCurrentModel,
      setModelsInitialized: state.setModelsInitialized,
      setIsLoading: state.setIsLoading,
      setIsRefreshing: state.setIsRefreshing,
      reset: state.reset,
    }))
  );
