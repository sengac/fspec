/**
 * Provider configuration management
 *
 * Provider settings (enabled, defaultModel, baseUrl, authMethod) are stored
 * in ~/.fspec/fspec-config.json under the "providers" key.
 *
 * This module contains interfaces and core config load/save functions.
 * Static registry data is in provider-registry.ts.
 * Profile management is in profile-management.ts.
 */

import { loadConfig, writeConfig, getFspecUserDir } from './config';
import { join } from 'path';
import { readFile } from 'fs/promises';

// Re-export everything from extracted modules for backward compatibility
export {
  SUPPORTED_PROVIDERS,
  type ProviderId,
  getProviderRegistry,
  getProviderRegistryEntry,
  isOAuthProvider,
} from './provider-registry';

export {
  loadProviderProfiles,
  saveProfile,
  deleteProfile,
  getProfile,
} from './profile-management';

/**
 * Provider authentication method
 */
export type AuthMethod = 'bearer' | 'x-api-key' | 'query_param' | 'none';

/**
 * Provider configuration
 */
export interface ProviderConfig {
  enabled?: boolean;
  baseUrl?: string;
  defaultModel?: string;
  authMethod?: AuthMethod;
  // Azure-specific
  endpoint?: string;
  apiVersion?: string;
  // Additional headers
  headers?: Record<string, string>;
  // PROV-007: Profile support
  profiles?: Record<string, ProfileConfig>;
}

/**
 * PROV-007: Profile configuration for local servers
 *
 * Profiles allow configuring multiple local LLM servers (vLLM, Ollama, etc.)
 * with their own baseUrl, apiKey, and optional settings.
 */
export interface ProfileConfig {
  /** API endpoint URL (e.g., "http://localhost:8888") */
  baseUrl: string;
  /** API key for this profile */
  apiKey: string;
  /** Context window size (optional) */
  contextWindow?: number;
  /** Max output tokens (optional) */
  maxOutputTokens?: number;
  /** MODEL-004: Custom models for this profile (optional, backward-compatible) */
  customModels?: CustomModelDefinition[];
}

/**
 * MODEL-004: Custom model definition for profile-based models.
 *
 * Allows users to manually register models that aren't listed by /v1/models,
 * configure per-model context windows, and override the facade type for
 * tool schema selection.
 */
export interface CustomModelDefinition {
  /** Model ID string sent to the API (required) */
  id: string;
  /** Human-readable display name (optional) */
  displayName?: string;
  /** Facade override for tool schema selection (optional) */
  facade?: 'openai' | 'codex' | 'claude' | 'gemini' | 'zai';
  /** Context window size in tokens (optional) */
  contextWindow?: number;
  /** Maximum output tokens (optional) */
  maxOutputTokens?: number;
  /** Whether model supports reasoning/thinking (optional) */
  reasoning?: boolean;
  /** Whether model supports vision/image input (optional) */
  hasVision?: boolean;
}

/**
 * Provider authentication type (credential acquisition strategy)
 * - 'api-key': Traditional API key authentication
 * - 'oauth': OAuth 2.0 flow (browser or device auth)
 */
export type AuthType = 'api-key' | 'oauth';

/**
 * Provider registry entry
 */
export interface ProviderRegistryEntry {
  id: string;
  name: string;
  baseUrl: string;
  envVar: string;
  authMethod: AuthMethod;
  authType: AuthType;
  requiresApiKey: boolean;
  description: string;
}

/**
 * Load provider configuration from user config
 *
 * @param providerId - The provider ID
 */
export async function loadProviderConfig(
  providerId: string
): Promise<ProviderConfig> {
  const config = await loadConfig();

  // Return provider config or empty object
  return config?.providers?.[providerId] || {};
}

/**
 * Save provider configuration to user config
 *
 * @param providerId - The provider ID
 * @param providerConfig - Configuration to save
 */
export async function saveProviderConfig(
  providerId: string,
  providerConfig: ProviderConfig
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
    // File doesn't exist, start with empty config
  }

  // Ensure providers object exists
  if (!config.providers) {
    config.providers = {};
  }

  const providers = config.providers as Record<string, Record<string, unknown>>;

  // Merge new config with existing provider config
  providers[providerId] = {
    ...providers[providerId],
    ...providerConfig,
  };

  // Write updated config
  await writeConfig('user', config);
}

/**
 * Check if a provider is configured (has required settings)
 */
export async function isProviderConfigured(
  providerId: string
): Promise<boolean> {
  const config = await loadProviderConfig(providerId);

  // Lazy import to avoid circular dependency
  const { getProviderRegistryEntry } = await import('./provider-registry');
  const registry = getProviderRegistryEntry(providerId);

  if (!registry) {
    return false;
  }

  // Providers that don't require API key (e.g., OpenAI API for local models)
  if (!registry.requiresApiKey) {
    return config.enabled !== false;
  }

  // Azure requires endpoint
  if (providerId === 'azure') {
    return !!config.endpoint && !!config.apiVersion;
  }

  // Other providers need credentials (checked separately via credentials.ts)
  return config.enabled !== false;
}

/**
 * Get all providers with their configuration status
 */
export async function getAllProvidersWithStatus(): Promise<
  Array<{
    id: string;
    name: string;
    configured: boolean;
    enabled: boolean;
    config: ProviderConfig;
  }>
> {
  // Lazy import to avoid circular dependency
  const { getProviderRegistryEntry, SUPPORTED_PROVIDERS: providers } =
    await import('./provider-registry');
  const results = [];

  for (const id of providers) {
    const entry = getProviderRegistryEntry(id);
    if (!entry) {
      continue;
    }
    const config = await loadProviderConfig(entry.id);
    const configured = await isProviderConfigured(entry.id);

    results.push({
      id: entry.id,
      name: entry.name,
      configured,
      enabled: config.enabled !== false,
      config,
    });
  }

  return results;
}
