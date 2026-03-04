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
import {
  loadCodexAllowlist,
  filterByCodexAllowlist,
} from './codexAllowlistService';
import type { CodexModelEntry } from './codexAllowlistService';

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
  // PROV-034: Load allowlist for filtering to Codex-supported models only
  if (hasCodexOAuth) {
    const codexAllowlist = await loadCodexAllowlist();
    return extractCodexSection(
      credentialSections,
      sectionsWithCreds,
      codexAllowlist
    );
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
 * PROV-018/PROV-033/PROV-034: Extract models from the OpenAI provider section
 * into a synthetic "Codex (ChatGPT)" section, filtered by the Codex allowlist.
 *
 * When Codex OAuth tokens exist, OpenAI cloud models are accessed via
 * Codex OAuth. PROV-033 removed the broken isCodexModel() filter.
 * PROV-034 adds allowlist-based filtering so only Codex-supported models
 * (from codex-models.json) appear — unsupported models like o3-pro,
 * gpt-4.1, etc. are hidden.
 *
 * @param credentialSections - Sections that already passed credentials check
 * @param allSections - All sections (including those without credentials)
 * @param codexAllowlist - Array of CodexModelEntry objects with slug, visibility, priority
 * @returns Updated sections array with synthetic Codex section, OpenAI section removed
 */
function extractCodexSection(
  credentialSections: ProviderSection[],
  allSections: ProviderSection[],
  codexAllowlist: CodexModelEntry[]
): ProviderSection[] {
  // Find the OpenAI section — may or may not have credentials
  const openaiSection = allSections.find(s => s.providerId === 'openai');

  if (!openaiSection || openaiSection.models.length === 0) {
    return credentialSections;
  }

  // PROV-034: Filter OpenAI models against the Codex allowlist using prefix matching
  const codexModels = filterByCodexAllowlist(
    openaiSection.models,
    codexAllowlist
  );

  if (codexModels.length === 0) {
    // No models match the allowlist — don't create an empty section
    return credentialSections.filter(s => s.providerId !== 'openai');
  }

  // Build the synthetic Codex section
  const codexSection: ProviderSection = {
    providerId: 'codex',
    providerName: 'Codex (ChatGPT)',
    internalName: mapProviderIdToInternal('codex'),
    models: codexModels,
    hasCredentials: true,
  };

  // PROV-033: Remove the OpenAI cloud section entirely — all models moved to Codex
  const filteredSections = credentialSections.filter(
    s => s.providerId !== 'openai'
  );

  logger.debug(
    `PROV-034: Extracted ${codexModels.length} models into synthetic Codex section (filtered from ${openaiSection.models.length} total)`
  );

  return [...filteredSections, codexSection];
}

/**
 * Load profile sections for local servers (vLLM, Ollama, etc.)
 */
async function loadProfileSections(): Promise<ProviderSection[]> {
  const profileSections: ProviderSection[] = [];

  for (const providerId of ['openai'] as const) {
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
