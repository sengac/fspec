/**
 * Provider configuration management
 *
 * Provider settings (enabled, defaultModel, baseUrl, authMethod) are stored
 * in ~/.fspec/fspec-config.json under the "providers" key.
 *
 * This module supports all 19 rig providers with their specific configuration needs.
 */

import { loadConfig, writeConfig, getFspecUserDir } from './config.js';
import { join } from 'path';
import { readFile } from 'fs/promises';

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
}

/**
 * Provider registry entry
 */
export interface ProviderRegistryEntry {
  id: string;
  name: string;
  baseUrl: string;
  envVar: string;
  authMethod: AuthMethod;
  requiresApiKey: boolean;
  description: string;
}

/**
 * All 19 supported rig providers
 */
export const SUPPORTED_PROVIDERS = [
  'openai',
  'anthropic',
  'cohere',
  'gemini',
  'mistral',
  'xai',
  'together',
  'huggingface',
  'openrouter',
  'groq',
  'ollama',
  'deepseek',
  'perplexity',
  'moonshot',
  'hyperbolic',
  'mira',
  'galadriel',
  'azure',
  'voyageai',
  'zai',
] as const;

export type ProviderId = (typeof SUPPORTED_PROVIDERS)[number];

/**
 * Provider registry with configuration details for each provider
 */
const PROVIDER_REGISTRY: ProviderRegistryEntry[] = [
  {
    id: 'openai',
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    envVar: 'OPENAI_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'OpenAI GPT models',
  },
  {
    id: 'anthropic',
    name: 'Anthropic',
    baseUrl: 'https://api.anthropic.com/v1',
    envVar: 'ANTHROPIC_API_KEY',
    authMethod: 'x-api-key',
    requiresApiKey: true,
    description: 'Anthropic Claude models',
  },
  {
    id: 'cohere',
    name: 'Cohere',
    baseUrl: 'https://api.cohere.ai/v1',
    envVar: 'COHERE_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Cohere language models',
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta',
    envVar: 'GOOGLE_GENERATIVE_AI_API_KEY',
    authMethod: 'query_param',
    requiresApiKey: true,
    description: 'Google Gemini models',
  },
  {
    id: 'mistral',
    name: 'Mistral AI',
    baseUrl: 'https://api.mistral.ai/v1',
    envVar: 'MISTRAL_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Mistral AI models',
  },
  {
    id: 'xai',
    name: 'xAI',
    baseUrl: 'https://api.x.ai/v1',
    envVar: 'XAI_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'xAI Grok models',
  },
  {
    id: 'together',
    name: 'Together AI',
    baseUrl: 'https://api.together.xyz/v1',
    envVar: 'TOGETHER_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Together AI hosted models',
  },
  {
    id: 'huggingface',
    name: 'Hugging Face',
    baseUrl: 'https://api-inference.huggingface.co/models',
    envVar: 'HUGGINGFACE_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Hugging Face inference API',
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    envVar: 'OPENROUTER_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'OpenRouter unified API',
  },
  {
    id: 'groq',
    name: 'Groq',
    baseUrl: 'https://api.groq.com/openai/v1',
    envVar: 'GROQ_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Groq fast inference',
  },
  {
    id: 'ollama',
    name: 'Ollama',
    baseUrl: 'http://localhost:11434',
    envVar: 'OLLAMA_API_KEY',
    authMethod: 'none',
    requiresApiKey: false,
    description: 'Local Ollama models',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com/v1',
    envVar: 'DEEPSEEK_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'DeepSeek models',
  },
  {
    id: 'perplexity',
    name: 'Perplexity',
    baseUrl: 'https://api.perplexity.ai',
    envVar: 'PERPLEXITY_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Perplexity AI models',
  },
  {
    id: 'moonshot',
    name: 'Moonshot',
    baseUrl: 'https://api.moonshot.cn/v1',
    envVar: 'MOONSHOT_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Moonshot AI models',
  },
  {
    id: 'hyperbolic',
    name: 'Hyperbolic',
    baseUrl: 'https://api.hyperbolic.xyz/v1',
    envVar: 'HYPERBOLIC_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Hyperbolic AI models',
  },
  {
    id: 'mira',
    name: 'Mira',
    baseUrl: 'https://api.mira.network/v1',
    envVar: 'MIRA_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Mira network models',
  },
  {
    id: 'galadriel',
    name: 'Galadriel',
    baseUrl: 'https://api.galadriel.com/v1',
    envVar: 'GALADRIEL_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Galadriel AI models',
  },
  {
    id: 'azure',
    name: 'Azure OpenAI',
    baseUrl: '', // Requires custom endpoint
    envVar: 'AZURE_OPENAI_API_KEY',
    authMethod: 'x-api-key',
    requiresApiKey: true,
    description: 'Azure OpenAI Service',
  },
  {
    id: 'voyageai',
    name: 'Voyage AI',
    baseUrl: 'https://api.voyageai.com/v1',
    envVar: 'VOYAGEAI_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description: 'Voyage AI embeddings',
  },
  {
    id: 'zai',
    name: 'Z.AI',
    baseUrl: 'https://api.z.ai/api/paas/v4',
    envVar: 'ZAI_API_KEY',
    authMethod: 'bearer',
    requiresApiKey: true,
    description:
      'Z.AI GLM models. Use ZAI_API_KEY for normal API, ZAI_PLAN_API_KEY for coding plan API (https://api.z.ai/api/coding/paas/v4)',
  },
];

/**
 * Get the provider registry (list of provider IDs)
 */
export function getProviderRegistry(): string[] {
  return [...SUPPORTED_PROVIDERS];
}

/**
 * Get detailed registry entry for a provider
 */
export function getProviderRegistryEntry(
  providerId: string
): ProviderRegistryEntry | undefined {
  return PROVIDER_REGISTRY.find(p => p.id === providerId);
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
  let config: any = {};

  try {
    const content = await readFile(userConfigPath, 'utf-8');
    if (content.trim()) {
      config = JSON.parse(content);
    }
  } catch {
    // File doesn't exist, start with empty config
  }

  // Ensure providers object exists
  if (!config.providers) {
    config.providers = {};
  }

  // Merge new config with existing provider config
  config.providers[providerId] = {
    ...config.providers[providerId],
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
  const registry = getProviderRegistryEntry(providerId);

  if (!registry) {
    return false;
  }

  // Ollama doesn't require API key
  if (!registry.requiresApiKey) {
    return config.enabled !== false; // Default to enabled for Ollama
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
  const results = [];

  for (const entry of PROVIDER_REGISTRY) {
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
