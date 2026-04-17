/**
 * Custom Model Utilities
 *
 * MODEL-004: Shared utility functions for custom model operations.
 * Extracted to eliminate duplication between useModelSelectorState.ts
 * and modelInitializationService.ts.
 */

import type { ProviderSection } from '../store/modelStore';
import type { CompactionThresholdConfig } from '../../utils/provider-config';

/**
 * Look up facade override for a model from custom model config.
 *
 * Custom models may have a facade override that changes which tool schemas
 * (names, parameter formats) are sent to the model via the Rust ProviderType dispatch.
 *
 * @param section - The provider section containing the model
 * @param modelId - The model ID to look up
 * @returns The facade override string if defined, undefined otherwise
 */
export function lookupFacadeOverride(
  section: ProviderSection,
  modelId: string
): string | undefined {
  if (!section.profileConfig?.customModels) {
    return undefined;
  }

  const customDef = section.profileConfig.customModels.find(
    c => c.id === modelId
  );

  return customDef?.facade;
}

/**
 * CTX-008: Look up compaction threshold for a model from custom model config,
 * falling back to the profile-level threshold.
 *
 * Priority: custom model threshold > profile threshold > undefined (use built-in)
 *
 * @param section - The provider section containing the model
 * @param modelId - The model ID to look up
 * @returns The compaction threshold config if defined, undefined otherwise
 */
export function lookupCompactionThreshold(
  section: ProviderSection,
  modelId: string
): CompactionThresholdConfig | undefined {
  if (section.profileConfig?.customModels) {
    const customDef = section.profileConfig.customModels.find(
      c => c.id === modelId
    );
    if (customDef?.compactionThreshold) {
      return customDef.compactionThreshold;
    }
  }

  return section.profileConfig?.compactionThreshold;
}
