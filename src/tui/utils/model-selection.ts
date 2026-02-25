/**
 * Model Selection Utilities
 *
 * PROV-007: Utilities for profile-aware model string handling.
 *
 * This module provides functions for:
 * - Building profile-qualified model strings for persistence
 * - Parsing persisted model strings to extract provider/profile/model
 * - Generating unique React keys for provider sections
 * - Finding sections that match persisted model strings
 *
 * Rules implemented:
 * - Rule [9]: Model selection string must include profile name for disambiguation
 * - Rule [10]: Profile section lookup must match by BOTH providerId AND profileName
 * - Rule [11]: React list keys must be unique - use 'section-{providerId}-{profileName || cloud}'
 */

import type { ProviderSection } from '../types/provider';

/**
 * Minimal section info needed for model string utilities
 * Allows these functions to work with both full ProviderSection and minimal objects
 */
export interface ProviderSectionInfo {
  providerId: string;
  profileName?: string;
}

/**
 * Parsed model string result
 */
export interface ParsedModelString {
  providerId: string;
  profileName: string | null;
  modelId: string;
}

/**
 * Build model string for persistence
 *
 * Rule [9]: Model selection string must include profile name for disambiguation:
 * - Profile section: 'provider:profile/modelId' (e.g., 'openai:work-vllm/Qwen3-80B')
 * - Cloud section: 'provider/modelId' (e.g., 'openai/gpt-4')
 *
 * @param section - The provider section info (must have providerId, optionally profileName)
 * @param modelId - The model ID
 * @returns The model string for persistence
 */
export function buildModelString(
  section: ProviderSectionInfo,
  modelId: string
): string {
  if (section.profileName) {
    // Profile section: use 'provider:profile/modelId' format
    return `${section.providerId}:${section.profileName}/${modelId}`;
  }
  // Cloud section: use 'provider/modelId' format
  return `${section.providerId}/${modelId}`;
}

/**
 * Parse model string to extract provider, profile, and model
 *
 * Handles both formats:
 * - Profile format: 'provider:profile/modelId' (e.g., 'openai:work-vllm/Qwen/Qwen3-80B')
 * - Cloud format: 'provider/modelId' (e.g., 'openai/gpt-4')
 *
 * Note: Model IDs can contain slashes (e.g., 'Qwen/Qwen3-80B'), so we use
 * the colon as the profile delimiter.
 *
 * @param modelString - The persisted model string
 * @returns Parsed result with providerId, profileName (or null), and modelId
 * @throws Error if the format is invalid
 */
export function parseModelString(modelString: string): ParsedModelString {
  // Check for profile format: 'provider:profile/modelId'
  // The colon separates provider:profile, then first slash starts the modelId
  const colonIndex = modelString.indexOf(':');
  const firstSlashIndex = modelString.indexOf('/');

  if (colonIndex !== -1 && colonIndex < firstSlashIndex) {
    // Profile format: extract provider, profile, and modelId
    const providerId = modelString.substring(0, colonIndex);
    const profileAndModel = modelString.substring(colonIndex + 1);
    const slashIndex = profileAndModel.indexOf('/');

    if (slashIndex === -1) {
      throw new Error(`Invalid model string format: ${modelString}`);
    }

    const profileName = profileAndModel.substring(0, slashIndex);
    const modelId = profileAndModel.substring(slashIndex + 1);

    return { providerId, profileName, modelId };
  }

  // Cloud format: 'provider/modelId'
  if (firstSlashIndex !== -1) {
    const providerId = modelString.substring(0, firstSlashIndex);
    const modelId = modelString.substring(firstSlashIndex + 1);

    return { providerId, profileName: null, modelId };
  }

  throw new Error(`Invalid model string format: ${modelString}`);
}

/**
 * Generate unique key for provider section (for React lists)
 *
 * Rule [11]: React list keys must be unique - use 'section-{providerId}-{profileName || cloud}'
 * to prevent duplicate keys when both cloud and profile sections exist for the same provider.
 *
 * @param section - The provider section info
 * @returns Unique key string for React
 */
export function generateSectionKey(section: ProviderSectionInfo): string {
  const suffix = section.profileName || 'cloud';
  return `section-${section.providerId}-${suffix}`;
}

/**
 * Find section matching persisted model string
 *
 * Rule [10]: Profile section lookup must match by BOTH providerId AND profileName
 * to avoid returning cloud provider section when profile was selected.
 *
 * @param sections - Array of provider sections
 * @param modelString - The persisted model string
 * @returns The matching section or null if not found
 */
export function findSectionForPersistedModel<T extends ProviderSectionInfo>(
  sections: T[],
  modelString: string
): T | null {
  try {
    const { providerId, profileName } = parseModelString(modelString);

    return (
      sections.find(s => {
        if (profileName) {
          // Profile was selected - must match BOTH providerId AND profileName
          return s.providerId === providerId && s.profileName === profileName;
        }
        // Cloud provider was selected - must match providerId AND NOT have profileName
        return s.providerId === providerId && !s.profileName;
      }) || null
    );
  } catch {
    // Invalid model string format
    return null;
  }
}
