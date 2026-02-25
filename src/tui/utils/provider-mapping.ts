/**
 * Provider Mapping Utilities
 *
 * PROV-008: Single source of truth for provider ID mappings.
 *
 * This module provides bidirectional mapping between:
 * - models.dev provider IDs (anthropic, google, openai)
 * - Internal provider names (claude, gemini, openai)
 * - Registry IDs for credential lookup
 *
 * DRY: All provider mapping logic consolidated here.
 * SRP: Only handles provider ID transformations.
 */

/**
 * Map models.dev provider ID to internal provider name
 *
 * Used for:
 * - Display in UI (provider tabs)
 * - Store state management
 * - Config persistence
 *
 * @example
 * mapProviderIdToInternal('anthropic') // 'claude'
 * mapProviderIdToInternal('google')    // 'gemini'
 * mapProviderIdToInternal('openai')    // 'openai'
 */
export function mapProviderIdToInternal(providerId: string): string {
  switch (providerId) {
    case 'anthropic':
      return 'claude';
    case 'google':
      return 'gemini';
    default:
      return providerId;
  }
}

/**
 * Map internal provider name to models.dev provider ID
 *
 * Reverse of mapProviderIdToInternal.
 *
 * Used for:
 * - API calls to models.dev
 * - Session creation with NAPI
 *
 * @example
 * mapInternalToProviderId('claude') // 'anthropic'
 * mapInternalToProviderId('gemini') // 'google'
 * mapInternalToProviderId('openai') // 'openai'
 */
export function mapInternalToProviderId(internalName: string): string {
  switch (internalName) {
    case 'claude':
      return 'anthropic';
    case 'gemini':
      return 'google';
    default:
      return internalName;
  }
}

/**
 * Map models.dev provider ID to registry ID for credential lookup
 *
 * The model registry uses different IDs than models.dev in some cases.
 *
 * Used for:
 * - Credential validation
 * - Registry API calls
 *
 * @example
 * mapModelsDevToRegistryId('google') // 'gemini'
 * mapModelsDevToRegistryId('anthropic') // 'anthropic'
 */
export function mapModelsDevToRegistryId(modelsDevProviderId: string): string {
  switch (modelsDevProviderId) {
    case 'google':
      return 'gemini';
    default:
      return modelsDevProviderId;
  }
}
