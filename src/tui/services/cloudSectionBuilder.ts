/**
 * Cloud Section Builder
 *
 * TUI-075: Builds provider sections from cloud models.dev data with
 * credential checks, Codex/Claude OAuth integration, and model filtering.
 *
 * Extracted from modelInitializationService.ts for separation of concerns
 * and to keep files under 300 lines.
 */

import {
  modelsListAll,
  codexOauthGetTokens,
  claudeOauthGetTokens,
} from '@sengac/codelet-napi';
import type { NapiModelInfo, NapiProviderModels } from '@sengac/codelet-napi';
import { getProviderRegistryEntry } from '../../utils/provider-config';
import { getProviderConfig } from '../../utils/credentials';
import { logger } from '../../utils/logger';
import type { ProviderSection } from '../store/modelStore';
import {
  mapProviderIdToInternal,
  mapModelsDevToRegistryId,
} from '../utils/provider-mapping';
import {
  loadCodexAllowlist,
  filterByCodexAllowlist,
} from './codexAllowlistService';
import type { CodexModelEntry } from './codexAllowlistService';

// =============================================================================
// OAUTH TOKEN CHECKS
// =============================================================================

/**
 * Check for Codex OAuth tokens (synchronous — reads from NAPI cache)
 */
function checkCodexOAuthTokens(): boolean {
  try {
    const tokens = codexOauthGetTokens();
    return !!tokens;
  } catch {
    return false;
  }
}

/**
 * Check for Claude OAuth tokens (async — claude_auth.json uses tokio::fs)
 */
async function checkClaudeOAuthTokens(): Promise<boolean> {
  try {
    const tokens = await claudeOauthGetTokens();
    return !!tokens;
  } catch {
    return false;
  }
}

// =============================================================================
// CLOUD MODEL LOADING
// =============================================================================

/**
 * Load cloud provider models from models.dev
 */
export async function loadCloudModels(): Promise<NapiProviderModels[]> {
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

// =============================================================================
// CLOUD SECTION BUILDING
// =============================================================================

/**
 * Build provider sections from cloud models with credentials check
 *
 * IMPORTANT: OpenAI cloud models (from models.dev) are ONLY accessible via
 * Codex credentials (OAuth tokens or CODEX_API_KEY). The 'openai' registry
 * entry has requiresApiKey: false because it's designed for local profiles
 * (vLLM, Ollama), but cloud models need explicit Codex credentials.
 * When Codex credentials exist, all OpenAI cloud models are shown under
 * "Codex (ChatGPT)" — never as a bare "OpenAI" section.
 */
export async function buildCloudSections(
  allModels: NapiProviderModels[]
): Promise<ProviderSection[]> {
  // PROV-018: Check for Codex OAuth tokens once, reuse across all providers
  const hasCodexOAuth = checkCodexOAuthTokens();
  // Check for Codex API key (CODEX_API_KEY env var or credentials file)
  const codexConfig = await getProviderConfig('codex');
  const hasCodexApiKey = !!codexConfig.apiKey;
  const hasCodexCredentials = hasCodexOAuth || hasCodexApiKey;
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

      // OpenAI cloud models require Codex credentials (OAuth or CODEX_API_KEY).
      // The 'openai' registry has requiresApiKey: false (for local profiles),
      // but cloud models must never appear without explicit Codex credentials.
      if (pm.providerId === 'openai') {
        hasCredentials = hasCodexCredentials;
      }

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

  // PROV-018/PROV-033: Extract OpenAI models into synthetic "Codex (ChatGPT)"
  // section when ANY Codex credentials exist (OAuth tokens OR CODEX_API_KEY).
  // PROV-034: Load allowlist for filtering to Codex-supported models only.
  const { codexSection, filteredSections } = await extractCodexSection(
    credentialSections,
    hasCodexCredentials
  );

  // Build final sections: codex first (if exists), then remaining providers
  const cloudSections: ProviderSection[] = [];

  if (codexSection) {
    cloudSections.push(codexSection);
  }

  for (const s of filteredSections) {
    cloudSections.push({
      providerId: s.providerId,
      providerName: s.providerName,
      internalName: s.internalName,
      models: s.models,
      hasCredentials: s.hasCredentials,
    });
  }

  return cloudSections;
}

// =============================================================================
// CODEX SECTION EXTRACTION
// =============================================================================

interface SectionWithCreds {
  providerId: string;
  providerName: string;
  internalName: string;
  models: NapiModelInfo[];
  hasCredentials: boolean;
}

/**
 * Extract OpenAI cloud models into a synthetic Codex section when OAuth credentials exist.
 *
 * PROV-018/PROV-033: When Codex credentials exist, ALL OpenAI cloud models
 * (including gpt-4o, o3, etc.) should appear under "Codex (ChatGPT)" — not
 * under a separate "OpenAI" section.
 *
 * PROV-034: Filters to only Codex-supported models when allowlist is available.
 */
async function extractCodexSection(
  sections: SectionWithCreds[],
  hasCodexCredentials: boolean
): Promise<{
  codexSection: ProviderSection | null;
  filteredSections: SectionWithCreds[];
}> {
  if (!hasCodexCredentials) {
    return { codexSection: null, filteredSections: sections };
  }

  // Find the OpenAI section (cloud models from models.dev)
  const openaiSection = sections.find(s => s.providerId === 'openai');
  const filtered = sections.filter(s => s.providerId !== 'openai');

  if (!openaiSection || openaiSection.models.length === 0) {
    return { codexSection: null, filteredSections: filtered };
  }

  // PROV-034: Load allowlist and filter models
  let codexModels: NapiModelInfo[] = openaiSection.models;
  let allowlist: CodexModelEntry[] = [];

  try {
    allowlist = await loadCodexAllowlist();
    if (allowlist.length > 0) {
      codexModels = filterByCodexAllowlist(openaiSection.models, allowlist);
      logger.debug(
        `Codex allowlist: ${openaiSection.models.length} → ${codexModels.length} models after filtering`
      );
    }
  } catch (err) {
    logger.warn(
      `Failed to load Codex allowlist, showing all OpenAI models: ${err instanceof Error ? err.message : String(err)}`
    );
  }

  const codexSection: ProviderSection = {
    providerId: 'codex',
    providerName: 'Codex (ChatGPT)',
    internalName: 'codex',
    models: codexModels,
    hasCredentials: true,
  };

  return { codexSection, filteredSections: filtered };
}
