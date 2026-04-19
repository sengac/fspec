/**
 * Custom Provider Section Builder
 *
 * PROV-067: Builds provider sections from discovered custom providers
 * (Rhai-scripted or facade-based) so they appear in the /model selector.
 *
 * Calls `listProviders()` via NAPI, filters to `isCustom && available`,
 * and converts each entry into a `ProviderSection` with synthetic
 * `NapiModelInfo` entries built from the config's model definitions.
 */

import { listProviders } from '@sengac/codelet-napi';
import type { JsProviderInfo } from '@sengac/codelet-napi';
import type { NapiModelInfo } from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import type { ProviderSection } from '../store/modelStore';

/**
 * Build NapiModelInfo from a custom provider's model alias.
 *
 * Custom provider configs declare models as alias → ModelDef with
 * `id`, `context_window`, `max_output_tokens`, etc. The NAPI
 * `listProviders()` only returns alias strings, so we build
 * minimal NapiModelInfo entries that the model selector can render.
 *
 * Context window and max output are populated later when the model
 * is selected via `sessionSetModelProfile`, which reads the full
 * config from disk on the Rust side.
 */
function buildCustomModelInfo(alias: string): NapiModelInfo {
  return {
    id: alias,
    name: alias,
    reasoning: false,
    toolCall: true,
    attachment: false,
    temperature: true,
    contextWindow: 128000,
    maxOutput: 8192,
    hasVision: false,
  };
}

/**
 * Check whether a providerId corresponds to a discovered custom provider
 * section. Used by modelSelectionService to route through
 * sessionSetModelProfile (bypasses registry validation).
 *
 * Reads from a module-level cache populated by loadCustomProviderSections().
 */
export function isCustomProviderSection(providerId: string): boolean {
  return discoveredCustomProviderIds.has(providerId);
}

/** Module-level cache of discovered custom provider slugs. */
let discoveredCustomProviderIds = new Set<string>();

/**
 * Load custom provider sections from discovered provider configs.
 *
 * Returns one ProviderSection per available custom provider. Each
 * section's models come from the config's model alias list.
 */
export async function loadCustomProviderSections(): Promise<ProviderSection[]> {
  try {
    const allProviders = await listProviders();
    const customProviders = allProviders.filter(
      (p: JsProviderInfo) => p.isCustom && p.available
    );

    if (customProviders.length === 0) {
      discoveredCustomProviderIds = new Set();
      return [];
    }

    logger.debug(
      `Found ${customProviders.length} available custom provider(s)`
    );

    const sections: ProviderSection[] = [];
    const ids = new Set<string>();

    for (const provider of customProviders) {
      const models: NapiModelInfo[] = provider.models.map(buildCustomModelInfo);

      if (models.length === 0) {
        logger.debug(
          `Skipping custom provider "${provider.name}": no models defined`
        );
        continue;
      }

      sections.push({
        providerId: provider.name,
        providerName: provider.displayName || provider.name,
        internalName: provider.name,
        models,
        hasCredentials: true,
      });

      ids.add(provider.name);
    }

    discoveredCustomProviderIds = ids;
    return sections;
  } catch (err) {
    logger.warn(
      `Failed to load custom provider sections: ${err instanceof Error ? err.message : String(err)}`
    );
    return [];
  }
}
