/**
 * Model Initialization Service
 *
 * TUI-075: Handles eager loading of models and restoration of persisted model selection.
 *
 * This service is responsible for:
 * 1. Loading models from models.dev (via NAPI)
 * 2. Loading profiles for local servers
 * 3. Restoring persisted model selection from config
 * 4. Setting default model if no persisted model available
 * 5. Updating the shared modelStore
 *
 * This runs on AgentView mount to ensure models are available for session creation.
 */

import type { NapiModelInfo } from '@sengac/codelet-napi';
import { loadConfig } from '../../utils/config';
import { logger } from '../../utils/logger';
import {
  useModelStore,
  type ProviderSection,
  type ModelSelection,
} from '../store/modelStore';
import { mapProviderIdToInternal } from '../utils/provider-mapping';
import {
  parseModelString,
  findSectionForPersistedModel,
} from '../utils/model-selection';
import { lookupFacadeOverride } from '../utils/custom-model-utils';
import { loadCloudModels, buildCloudSections } from './cloudSectionBuilder';
import { loadProfileSections } from './profileSectionBuilder';

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/**
 * Extract model ID for registry lookup (strips date suffix)
 * e.g., "claude-sonnet-4-20250514" -> "claude-sonnet-4"
 */
export const extractModelIdForRegistry = (modelId: string): string => {
  // Match pattern: name-YYYYMMDD at the end
  const match = modelId.match(/^(.+)-(\d{8})$/);
  if (match) {
    return match[1];
  }
  return modelId;
};

// =============================================================================
// TYPES
// =============================================================================

export interface ModelInitializationResult {
  sections: ProviderSection[];
  currentModel: ModelSelection | null;
  currentProvider: string | null;
  availableProviders: string[];
  persistedModelRestored: boolean;
}

// =============================================================================
// MODEL SELECTION HELPERS
// =============================================================================

/**
 * Load persisted model string from config
 */
async function loadPersistedModelString(): Promise<string | null> {
  try {
    const config = await loadConfig();
    return config?.tui?.lastUsedModel || null;
  } catch (err) {
    logger.error('Failed to load config for persisted model', { error: err });
    return null;
  }
}

/**
 * Create a ModelSelection from section and model info
 */
function createModelSelection(
  section: ProviderSection,
  model: NapiModelInfo
): ModelSelection {
  const facade = lookupFacadeOverride(section, model.id);

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
  };
}

/**
 * Select default model (first available with models)
 */
function selectDefaultModel(
  sections: ProviderSection[]
): { section: ProviderSection; model: NapiModelInfo } | null {
  for (const section of sections) {
    if (section.models.length > 0) {
      return { section, model: section.models[0] };
    }
  }
  return null;
}

/**
 * Restore persisted model from sections
 */
function restorePersistedModel(
  persistedModelString: string,
  sections: ProviderSection[]
): { currentModel: ModelSelection; currentProvider: string } | null {
  try {
    const parsed = parseModelString(persistedModelString);
    const section = findSectionForPersistedModel(
      sections,
      persistedModelString
    );

    if (section && section.hasCredentials) {
      const normalizedModelId = extractModelIdForRegistry(parsed.modelId);
      const model = section.models.find(
        m => extractModelIdForRegistry(m.id) === normalizedModelId
      );

      if (model) {
        return {
          currentModel: createModelSelection(section, model),
          currentProvider: section.internalName,
        };
      }
    }
  } catch (err) {
    logger.error('Invalid persisted model string format', {
      modelString: persistedModelString,
      error: err,
    });
  }
  return null;
}

// =============================================================================
// MAIN INITIALIZATION FUNCTION
// =============================================================================

/**
 * Initialize models: load from NAPI, restore persisted selection, update store.
 *
 * This is the main entry point called from AgentView's initSession effect.
 *
 * @returns ModelInitializationResult with all loaded data
 */
export async function initializeModels(): Promise<ModelInitializationResult> {
  const store = useModelStore.getState();

  // Skip if already initialized
  if (store.modelsInitialized) {
    return {
      sections: store.providerSections,
      currentModel: store.currentModel,
      currentProvider: store.currentModel
        ? mapProviderIdToInternal(store.currentModel.providerId)
        : null,
      availableProviders: store.providerSections
        .filter(s => s.models.length > 0)
        .map(s => s.internalName),
      persistedModelRestored: false,
    };
  }

  store.setIsLoading(true);

  try {
    // Load cloud and profile models in parallel
    const [cloudModels, profileSections] = await Promise.all([
      loadCloudModels(),
      loadProfileSections(),
    ]);
    const cloudSections = await buildCloudSections(cloudModels);

    // Combine: profiles first, then cloud; filter out unreachable + 0-model sections
    const sections: ProviderSection[] = [
      ...profileSections,
      ...cloudSections,
    ].filter(s => !s.isUnreachable || s.models.length > 0);

    // Load persisted model string
    const persistedModelString = await loadPersistedModelString();

    // Try to restore persisted model
    let currentModel: ModelSelection | null = null;
    let currentProvider: string | null = null;
    let persistedModelRestored = false;

    if (persistedModelString) {
      const restored = restorePersistedModel(persistedModelString, sections);
      if (restored) {
        currentModel = restored.currentModel;
        currentProvider = restored.currentProvider;
        persistedModelRestored = true;
      }
    }

    // Fall back to first available model if no persisted model
    if (!currentModel) {
      const defaultSelection = selectDefaultModel(sections);
      if (defaultSelection) {
        currentModel = createModelSelection(
          defaultSelection.section,
          defaultSelection.model
        );
        currentProvider = defaultSelection.section.internalName;
      }
    }

    // Update store
    store.setProviderSections(sections);
    if (currentModel) {
      store.setCurrentModel(currentModel);
    }
    store.setModelsInitialized(true);

    const availableProviders = sections
      .filter(s => s.models.length > 0)
      .map(s => s.internalName);

    return {
      sections,
      currentModel,
      currentProvider,
      availableProviders,
      persistedModelRestored,
    };
  } catch (err) {
    logger.error('Failed to initialize models:', err);
    return {
      sections: [],
      currentModel: null,
      currentProvider: null,
      availableProviders: [],
      persistedModelRestored: false,
    };
  } finally {
    store.setIsLoading(false);
  }
}
