/**
 * Custom Model Utilities
 *
 * MODEL-004: Shared utility functions for custom model operations.
 * Extracted to eliminate duplication between useModelSelectorState.ts
 * and modelInitializationService.ts.
 */

import type { ProviderSection } from '../store/modelStore';

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
