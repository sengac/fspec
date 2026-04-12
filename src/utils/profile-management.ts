/**
 * Profile Management — Load, save, and delete provider profiles
 *
 * Profiles allow configuring multiple local LLM servers (vLLM, Ollama, etc.)
 * with their own baseUrl, apiKey, and optional settings.
 *
 * Extracted from provider-config.ts to maintain separation of concerns
 * and keep files under 300 lines.
 */

import { join } from 'path';
import { readFile } from 'fs/promises';
import { writeConfig, getFspecUserDir } from './config';
import type { ProfileConfig, ProviderConfig } from './provider-config';
import { loadProviderConfig } from './provider-config';

/**
 * Load all profiles for a provider
 *
 * @param providerId - The provider ID (e.g., "openai")
 * @returns Record of profile name to ProfileConfig
 */
export async function loadProviderProfiles(
  providerId: string
): Promise<Record<string, ProfileConfig>> {
  const config = await loadProviderConfig(providerId);
  return config.profiles || {};
}

/**
 * Save a profile for a provider
 *
 * Creates or updates a profile with the given name.
 *
 * @param providerId - The provider ID (e.g., "openai")
 * @param profileName - The profile name (e.g., "work-vllm")
 * @param profileConfig - The profile configuration
 */
export async function saveProfile(
  providerId: string,
  profileName: string,
  profileConfig: ProfileConfig
): Promise<void> {
  // Guard: profiles are only supported for OpenAI API provider
  if (providerId !== 'openai') {
    throw new Error('Profiles are only supported for OpenAI API provider');
  }

  // Load existing user config
  const userConfigPath = join(getFspecUserDir(), 'fspec-config.json');
  let config: Record<string, unknown> = {};

  try {
    const content = await readFile(userConfigPath, 'utf-8');
    if (content.trim()) {
      config = JSON.parse(content) as Record<string, unknown>;
    }
  } catch {
    // File doesn't exist, start with empty config
  }

  // Ensure providers object exists
  if (!config.providers) {
    config.providers = {};
  }

  const providers = config.providers as Record<string, Record<string, unknown>>;

  // Ensure provider object exists
  if (!providers[providerId]) {
    providers[providerId] = {};
  }

  // Ensure profiles object exists
  if (!providers[providerId].profiles) {
    providers[providerId].profiles = {};
  }

  const profiles = providers[providerId].profiles as Record<
    string,
    ProfileConfig
  >;

  // Save the profile
  profiles[profileName] = profileConfig;

  // Write updated config
  await writeConfig('user', config);
}

/**
 * Delete a profile from a provider
 *
 * @param providerId - The provider ID (e.g., "openai")
 * @param profileName - The profile name to delete
 */
export async function deleteProfile(
  providerId: string,
  profileName: string
): Promise<void> {
  // Load existing user config
  const userConfigPath = join(getFspecUserDir(), 'fspec-config.json');
  let config: Record<string, unknown> = {};

  try {
    const content = await readFile(userConfigPath, 'utf-8');
    if (content.trim()) {
      config = JSON.parse(content) as Record<string, unknown>;
    }
  } catch {
    // File doesn't exist, nothing to delete
    return;
  }

  const providers = config.providers as
    | Record<string, Record<string, unknown>>
    | undefined;
  if (!providers?.[providerId]?.profiles) {
    return;
  }

  const profiles = providers[providerId].profiles as Record<
    string,
    ProfileConfig
  >;

  // Delete the profile
  delete profiles[profileName];

  // Write updated config
  await writeConfig('user', config);
}

/**
 * Get a specific profile for a provider
 *
 * @param providerId - The provider ID (e.g., "openai")
 * @param profileName - The profile name
 * @returns The profile configuration or undefined if not found
 */
export async function getProfile(
  providerId: string,
  profileName: string
): Promise<ProfileConfig | undefined> {
  const profiles = await loadProviderProfiles(providerId);
  return profiles[profileName];
}
