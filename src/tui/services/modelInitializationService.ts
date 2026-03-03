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

import {
  modelsListAll,
  modelsListLocalOpenai,
  codexOauthGetTokens,
  claudeOauthGetTokens,
} from '@sengac/codelet-napi';
import type { NapiModelInfo, NapiProviderModels } from '@sengac/codelet-napi';
import {
  loadProviderProfiles,
  getProviderRegistryEntry,
  SUPPORTED_PROVIDERS,
} from '../../utils/provider-config';
import { getProviderConfig } from '../../utils/credentials';
import { loadConfig } from '../../utils/config';
import { logger } from '../../utils/logger';
import {
  useModelStore,
  type ProviderSection,
  type ModelSelection,
} from '../store/modelStore';
import {
  mapProviderIdToInternal,
  mapModelsDevToRegistryId,
} from '../utils/provider-mapping';
import {
  parseModelString,
  findSectionForPersistedModel,
} from '../utils/model-selection';

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

/**
 * PROV-018: Check if a model ID belongs to the Codex family.
 * Codex models are identified by their ID containing 'codex' (case-insensitive).
 */
export const isCodexModel = (modelId: string): boolean => {
  return modelId.toLowerCase().includes('codex');
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
// CORE SERVICE FUNCTIONS
// =============================================================================

/**
 * Load cloud provider models from models.dev
 */
async function loadCloudModels(): Promise<NapiProviderModels[]> {
  try {
    const allModels = await modelsListAll();
    logger.debug(`Loaded ${allModels.length} providers from models.dev`);
    return allModels;
  } catch (err) {
    const errorMsg = err instanceof Error ? err.message : String(err);
    logger.error(`Failed to load models from models.dev: ${errorMsg}`);
    return [];
  }
}

/**
 * Build provider sections from cloud models with credentials check
 */
async function buildCloudSections(
  allModels: NapiProviderModels[]
): Promise<ProviderSection[]> {
  // PROV-018: Check for Codex OAuth tokens once, reuse across all providers
  const hasCodexOAuth = checkCodexOAuthTokens();
  // PROV-026: Check for Claude OAuth tokens (async — claude_auth.json uses tokio::fs)
  const hasClaudeOAuth = await checkClaudeOAuthTokens();

  const sectionsWithCreds = await Promise.all(
    allModels.map(async pm => {
      const internalName = mapProviderIdToInternal(pm.providerId);
      const registryId = mapModelsDevToRegistryId(pm.providerId);
      const registryEntry = getProviderRegistryEntry(registryId);
      const providerConfig = await getProviderConfig(registryId);
      let hasCredentials =
        registryEntry?.requiresApiKey === false || !!providerConfig.apiKey;
      const toolCallModels = pm.models.filter(m => m.toolCall);

      // PROV-026: Override hasCredentials for anthropic when Claude OAuth tokens exist
      if (pm.providerId === 'anthropic' && hasClaudeOAuth) {
        hasCredentials = true;
      }

      logger.debug(
        `Provider ${pm.providerId}: registryId=${registryId}, hasApiKey=${!!providerConfig.apiKey}, source=${providerConfig.source}, hasCredentials=${hasCredentials}`
      );

      return {
        providerId: pm.providerId,
        providerName: pm.providerName,
        internalName,
        models: toolCallModels,
        hasCredentials,
      };
    })
  );

  const credentialSections = sectionsWithCreds.filter(s => s.hasCredentials);

  // PROV-018: Extract codex models from OpenAI section when OAuth tokens exist
  if (hasCodexOAuth) {
    return extractCodexSection(credentialSections, sectionsWithCreds);
  }

  return credentialSections;
}

/**
 * PROV-018: Check if Codex OAuth tokens exist.
 * Pure boolean check — isolates NAPI call for testability.
 */
function checkCodexOAuthTokens(): boolean {
  try {
    const tokens = codexOauthGetTokens();
    return tokens !== null && tokens !== undefined;
  } catch {
    return false;
  }
}

/**
 * PROV-026: Check if Claude OAuth tokens exist.
 * Async because claudeOauthGetTokens() reads claude_auth.json via tokio::fs.
 * Returns true if tokens exist, false otherwise.
 */
async function checkClaudeOAuthTokens(): Promise<boolean> {
  try {
    const tokens = await claudeOauthGetTokens();
    return tokens !== null && tokens !== undefined;
  } catch {
    return false;
  }
}

/**
 * PROV-018: Extract codex models from the OpenAI provider section
 * into a synthetic "Codex (ChatGPT)" section.
 *
 * When Codex OAuth tokens exist:
 * 1. Find the OpenAI section (from all sections, even those without API keys)
 * 2. Split its models into codex vs non-codex
 * 3. Create a synthetic Codex section with providerId='codex'
 * 4. If OpenAI also has credentials, keep non-codex models in OpenAI section
 *
 * @param credentialSections - Sections that already passed credentials check
 * @param allSections - All sections (including those without credentials)
 * @returns Updated sections array with synthetic Codex section appended
 */
function extractCodexSection(
  credentialSections: ProviderSection[],
  allSections: ProviderSection[]
): ProviderSection[] {
  // Find the OpenAI section — may or may not have credentials
  const openaiSection = allSections.find(s => s.providerId === 'openai');

  if (!openaiSection) {
    return credentialSections;
  }

  const codexModels = openaiSection.models.filter(m => isCodexModel(m.id));

  if (codexModels.length === 0) {
    return credentialSections;
  }

  // Build the synthetic Codex section
  const codexSection: ProviderSection = {
    providerId: 'codex',
    providerName: 'Codex (ChatGPT)',
    internalName: mapProviderIdToInternal('codex'),
    models: codexModels,
    hasCredentials: true,
  };

  // If OpenAI section exists in credentialSections, strip codex models from it
  const updatedSections = credentialSections.map(s => {
    if (s.providerId === 'openai') {
      return {
        ...s,
        models: s.models.filter(m => !isCodexModel(m.id)),
      };
    }
    return s;
  });

  // Remove empty OpenAI section (if all models were codex)
  const filteredSections = updatedSections.filter(
    s => s.providerId !== 'openai' || s.models.length > 0
  );

  logger.debug(
    `PROV-018: Extracted ${codexModels.length} codex models into synthetic Codex section`
  );

  return [...filteredSections, codexSection];
}

/**
 * Load profile sections for local servers (vLLM, Ollama, etc.)
 */
async function loadProfileSections(): Promise<ProviderSection[]> {
  const profileSections: ProviderSection[] = [];

  for (const providerId of SUPPORTED_PROVIDERS) {
    try {
      const profiles = await loadProviderProfiles(providerId);
      const profileNames = Object.keys(profiles);

      for (const profileName of profileNames) {
        const profile = profiles[profileName];
        const displayName = `${providerId}: ${profileName}`;

        // Fetch models from local server
        let localModels: NapiModelInfo[] = [];
        let isUnreachable = false;

        try {
          const modelIds = await modelsListLocalOpenai(profile.baseUrl);
          localModels = modelIds.map(id => ({
            id,
            name: id,
            reasoning: false,
            toolCall: true,
            attachment: false,
            temperature: true,
            contextWindow: profile.contextWindow || 128000,
            maxOutput: profile.maxOutputTokens || 16384,
            hasVision: false,
          }));
        } catch (err) {
          logger.warn(
            `Failed to fetch models from ${profile.baseUrl}: ${err instanceof Error ? err.message : String(err)}`
          );
          isUnreachable = true;
        }

        profileSections.push({
          providerId,
          providerName: isUnreachable
            ? `${displayName} (unreachable)`
            : displayName,
          internalName: mapProviderIdToInternal(providerId),
          models: localModels,
          hasCredentials: true,
          profileName,
          profileConfig: profile,
          isUnreachable,
        });
      }
    } catch (err) {
      logger.warn(
        `Failed to load profiles for ${providerId}: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  }

  return profileSections;
}

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
 * Find a model in sections by provider ID and model ID
 */
function findModelInSections(
  sections: ProviderSection[],
  providerId: string,
  modelId: string
): { section: ProviderSection; model: NapiModelInfo } | null {
  const section = sections.find(s => s.providerId === providerId);
  if (!section || !section.hasCredentials) {
    return null;
  }

  // Compare both sides using extractModelIdForRegistry to handle date suffixes
  const normalizedModelId = extractModelIdForRegistry(modelId);
  const model = section.models.find(
    m => extractModelIdForRegistry(m.id) === normalizedModelId
  );
  if (!model) {
    return null;
  }

  return { section, model };
}

/**
 * Create a ModelSelection from section and model info
 */
function createModelSelection(
  section: ProviderSection,
  model: NapiModelInfo
): ModelSelection {
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
    // Load cloud models
    const cloudModels = await loadCloudModels();
    const cloudSections = await buildCloudSections(cloudModels);

    // Load profile sections
    const profileSections = await loadProfileSections();

    // Combine: profiles first, then cloud
    const sections: ProviderSection[] = [...profileSections, ...cloudSections];

    // Load persisted model string
    const persistedModelString = await loadPersistedModelString();

    // Try to restore persisted model
    let currentModel: ModelSelection | null = null;
    let currentProvider: string | null = null;
    let persistedModelRestored = false;

    if (persistedModelString) {
      try {
        // BUG-097: Use parseModelString to correctly handle profile format
        // 'provider:profile/modelId' (e.g., 'openai:work-vllm/Qwen/Qwen3-80B')
        const parsed = parseModelString(persistedModelString);

        // BUG-097: Use findSectionForPersistedModel to match by BOTH providerId AND profileName
        const section = findSectionForPersistedModel(
          sections,
          persistedModelString
        );

        if (section && section.hasCredentials) {
          // Find the model within the section
          const normalizedModelId = extractModelIdForRegistry(parsed.modelId);
          const model = section.models.find(
            m => extractModelIdForRegistry(m.id) === normalizedModelId
          );

          if (model) {
            currentModel = createModelSelection(section, model);
            currentProvider = section.internalName;
            persistedModelRestored = true;
          }
        }
      } catch (err) {
        logger.error('Invalid persisted model string format', {
          modelString: persistedModelString,
          error: err,
        });
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
