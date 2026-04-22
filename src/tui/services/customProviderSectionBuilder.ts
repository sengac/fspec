/**
 * Custom Provider Section Builder
 *
 * PROV-067: Builds provider sections from discovered custom providers
 * (Rhai-scripted or facade-based) so they appear in the /model selector.
 *
 * Calls `listProviders()` via NAPI, filters to `isCustom && available`,
 * and converts each entry into a `ProviderSection` with synthetic
 * `NapiModelInfo` entries built from the config's model definitions.
 *
 * BUG-139: The NAPI `listProviders()` return shape was widened so each
 * entry in `provider.models` is now a `JsProviderModelInfo` object
 * (id + contextWindow + maxOutput + supports_* flags) rather than a
 * bare alias string. This lets us populate `NapiModelInfo.contextWindow`
 * / `.maxOutput` / `.toolCall` / `.reasoning` from authoritative
 * provider-JSON values instead of the legacy `128000 / 8192` fallback
 * that caused the SessionHeader badge to display a stale `[120k]`.
 */

import { listProviders } from '@sengac/codelet-napi';
import type { JsProviderInfo, JsProviderModelInfo } from '@sengac/codelet-napi';
import type { NapiModelInfo } from '@sengac/codelet-napi';
import { logger } from '../../utils/logger';
import type { ProviderSection } from '../store/modelStore';

/**
 * Build `NapiModelInfo` from a per-model entry on `JsProviderInfo.models`.
 *
 * BUG-139: Previously this function received the raw alias string and
 * hardcoded `contextWindow=128000 / maxOutput=8192`. It now consumes
 * the widened `JsProviderModelInfo` shape and forwards:
 *   - `contextWindow` <- `entry.contextWindow`
 *   - `maxOutput`     <- `entry.maxOutput`
 *   - `toolCall`      <- `entry.supportsTools`
 *   - `reasoning`     <- `entry.supportsThinking`
 *
 * PROV-096: `hasVision` now forwards `entry.supportsVision` (default false)
 * so Rhai-scripted / custom provider models that declare `supports_vision`
 * in their JSON config correctly show the `[V]` badge in the SessionHeader.
 *
 * @param entry Per-model info from `JsProviderInfo.models`.
 * @returns A `NapiModelInfo` whose limits match the provider JSON.
 */
function buildCustomModelInfo(entry: JsProviderModelInfo): NapiModelInfo {
  return {
    id: entry.id,
    name: entry.id,
    reasoning: entry.supportsThinking,
    toolCall: entry.supportsTools,
    attachment: false,
    temperature: true,
    contextWindow: entry.contextWindow,
    maxOutput: entry.maxOutput,
    hasVision: entry.supportsVision ?? false,
  };
}

/**
 * Check whether a providerId corresponds to a discovered custom provider
 * section. Used by modelSelectionService to route through
 * sessionSetModelProfile (bypasses registry validation).
 *
 * Reads from a module-level cache populated by loadCustomProviderSections().
 *
 * @param providerId Provider slug to check.
 * @returns `true` when the slug matches a discovered custom provider.
 */
export function isCustomProviderSection(providerId: string): boolean {
  return discoveredCustomProviderIds.has(providerId);
}

/** Module-level cache of discovered custom provider slugs. */
let discoveredCustomProviderIds = new Set<string>();

/**
 * Load custom provider sections from discovered provider configs.
 *
 * Returns one `ProviderSection` per available custom provider. Each
 * section's models come from the widened `JsProviderInfo.models`
 * returned by the NAPI `listProviders()` binding — so per-model
 * `contextWindow` / `maxOutput` / `supports_*` values flow through
 * verbatim from the provider's JSON config.
 *
 * @returns Array of `ProviderSection` entries ready for the model selector.
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
