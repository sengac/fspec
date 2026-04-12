/**
 * Profile Section Builder
 *
 * MODEL-004: Handles loading profile sections from local server configurations
 * (vLLM, Ollama, LiteLLM, etc.), including custom model merging.
 *
 * Extracted from modelInitializationService.ts for separation of concerns
 * and file size compliance (< 300 lines).
 */

import { modelsListLocalOpenai } from '@sengac/codelet-napi';
import type { NapiModelInfo } from '@sengac/codelet-napi';
import { loadProviderProfiles } from '../../utils/provider-config';
import { logger } from '../../utils/logger';
import type { ProviderSection } from '../store/modelStore';
import { mapProviderIdToInternal } from '../utils/provider-mapping';

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/**
 * Build NapiModelInfo from a custom model definition and profile defaults.
 */
function buildCustomModelInfo(
  custom: {
    id: string;
    displayName?: string;
    reasoning?: boolean;
    hasVision?: boolean;
    contextWindow?: number;
    maxOutputTokens?: number;
  },
  profileContextWindow: number,
  profileMaxOutput: number
): NapiModelInfo {
  return {
    id: custom.id,
    name: custom.displayName || custom.id,
    reasoning: custom.reasoning || false,
    toolCall: true,
    attachment: false,
    temperature: true,
    contextWindow: custom.contextWindow || profileContextWindow,
    maxOutput: custom.maxOutputTokens || profileMaxOutput,
    hasVision: custom.hasVision || false,
  };
}

/**
 * Merge custom models into the auto-discovered local model list.
 * Custom models override auto-discovered models with matching IDs.
 * Returns the merged model list and a Set of custom model IDs.
 */
function mergeCustomModels(
  localModels: NapiModelInfo[],
  customModels: Array<{
    id: string;
    displayName?: string;
    reasoning?: boolean;
    hasVision?: boolean;
    contextWindow?: number;
    maxOutputTokens?: number;
  }>,
  profileContextWindow: number,
  profileMaxOutput: number
): { models: NapiModelInfo[]; customModelIds: Set<string> } {
  const customModelIds = new Set<string>();
  const models = [...localModels];

  for (const custom of customModels) {
    customModelIds.add(custom.id);

    const customNapiModel = buildCustomModelInfo(
      custom,
      profileContextWindow,
      profileMaxOutput
    );

    const existingIdx = models.findIndex(m => m.id === custom.id);
    if (existingIdx !== -1) {
      // Override auto-discovered model with custom metadata
      models[existingIdx] = customNapiModel;
    } else {
      // Add new custom model
      models.push(customNapiModel);
    }
  }

  return { models, customModelIds };
}

// =============================================================================
// MAIN FUNCTION
// =============================================================================

/**
 * Load profile sections from local server configurations (vLLM, Ollama, etc.)
 *
 * MODEL-004: Also loads custom models from profile config and merges them
 * into the auto-discovered model list.
 *
 * PROV-040: Now passes API key to modelsListLocalOpenai for servers that
 * require authentication (e.g., Fireworks AI, Together AI).
 */
export async function loadProfileSections(): Promise<ProviderSection[]> {
  const profileSections: ProviderSection[] = [];

  for (const providerId of ['openai'] as const) {
    try {
      const profiles = await loadProviderProfiles(providerId);
      const profileNames = Object.keys(profiles);

      for (const profileName of profileNames) {
        const profile = profiles[profileName];
        const displayName = `${providerId}: ${profileName}`;
        const profileContextWindow = profile.contextWindow || 128000;
        const profileMaxOutput = profile.maxOutputTokens || 16384;

        // Fetch models from local server
        let localModels: NapiModelInfo[] = [];
        let isUnreachable = false;

        try {
          // PROV-040: Pass API key for servers requiring authentication
          const modelIds = await modelsListLocalOpenai(
            profile.baseUrl,
            profile.apiKey || null
          );
          localModels = modelIds.map(id => ({
            id,
            name: id,
            reasoning: false,
            toolCall: true,
            attachment: false,
            temperature: true,
            contextWindow: profileContextWindow,
            maxOutput: profileMaxOutput,
            hasVision: false,
          }));
        } catch (err) {
          logger.warn(
            `Failed to fetch models from ${profile.baseUrl}: ${err instanceof Error ? err.message : String(err)}`
          );
          isUnreachable = true;
        }

        // MODEL-004: Load custom models from profile config and merge
        const customModels = profile.customModels || [];
        const { models: mergedModels, customModelIds } = mergeCustomModels(
          localModels,
          customModels,
          profileContextWindow,
          profileMaxOutput
        );

        // MODEL-004: If custom models exist, section is NOT unreachable
        const hasCustomModels = customModels.length > 0;
        const effectiveUnreachable = isUnreachable && !hasCustomModels;

        profileSections.push({
          providerId,
          providerName: effectiveUnreachable
            ? `${displayName} (unreachable)`
            : displayName,
          internalName: mapProviderIdToInternal(providerId),
          models: mergedModels,
          hasCredentials: true,
          profileName,
          profileConfig: profile,
          isUnreachable: effectiveUnreachable,
          customModelIds: customModelIds.size > 0 ? customModelIds : undefined,
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
