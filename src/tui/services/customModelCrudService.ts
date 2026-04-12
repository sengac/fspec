/**
 * Custom Model CRUD Service
 *
 * MODEL-004: Read-modify-write operations for custom models in profile config.
 * Custom models are stored in fspec-config.json under:
 *   providers → openai → profiles → <profileName> → customModels[]
 *
 * All operations use the existing saveProfile/getProfile API from
 * profile-management.ts — no new storage mechanism needed.
 */

import { getProfile, saveProfile } from '../../utils/provider-config';
import type {
  CustomModelDefinition,
  ProfileConfig,
} from '../../utils/provider-config';
import { logger } from '../../utils/logger';

/**
 * Add or update a custom model in a profile.
 *
 * If originalModelId is provided (edit mode), replaces the model with that ID.
 * Otherwise, appends the new model to the customModels array.
 *
 * @param providerId - Provider ID (e.g., 'openai')
 * @param profileName - Profile name (e.g., 'work-vllm')
 * @param definition - The custom model definition to save
 * @param originalModelId - If editing, the original model ID to replace
 */
export async function saveCustomModel(
  providerId: string,
  profileName: string,
  definition: CustomModelDefinition,
  originalModelId?: string
): Promise<void> {
  const profile = await getProfile(providerId, profileName);
  if (!profile) {
    logger.warn(
      `Cannot save custom model: profile "${profileName}" not found for "${providerId}"`
    );
    return;
  }

  const existingCustomModels: CustomModelDefinition[] =
    profile.customModels || [];

  let updatedCustomModels: CustomModelDefinition[];

  if (originalModelId) {
    // Edit mode: replace the model with matching ID
    updatedCustomModels = existingCustomModels.map(m =>
      m.id === originalModelId ? definition : m
    );
  } else {
    // Add mode: append to the array
    updatedCustomModels = [...existingCustomModels, definition];
  }

  const updatedProfile: ProfileConfig = {
    ...profile,
    customModels: updatedCustomModels,
  };

  await saveProfile(providerId, profileName, updatedProfile);
  logger.debug(
    `Saved custom model "${definition.id}" to profile "${profileName}"`
  );
}

/**
 * Delete a custom model from a profile by its ID.
 *
 * @param providerId - Provider ID (e.g., 'openai')
 * @param profileName - Profile name (e.g., 'work-vllm')
 * @param modelId - The custom model ID to delete
 */
export async function deleteCustomModel(
  providerId: string,
  profileName: string,
  modelId: string
): Promise<void> {
  const profile = await getProfile(providerId, profileName);
  if (!profile) {
    logger.warn(
      `Cannot delete custom model: profile "${profileName}" not found for "${providerId}"`
    );
    return;
  }

  const existingCustomModels: CustomModelDefinition[] =
    profile.customModels || [];

  const updatedCustomModels = existingCustomModels.filter(
    m => m.id !== modelId
  );

  const updatedProfile: ProfileConfig = {
    ...profile,
    customModels:
      updatedCustomModels.length > 0 ? updatedCustomModels : undefined,
  };

  await saveProfile(providerId, profileName, updatedProfile);
  logger.debug(
    `Deleted custom model "${modelId}" from profile "${profileName}"`
  );
}
