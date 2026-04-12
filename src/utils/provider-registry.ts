/**
 * Provider Registry — Static provider metadata
 *
 * Contains the complete list of supported providers with their configuration
 * details (base URLs, env vars, auth methods). Extracted from provider-config.ts
 * to keep files under 300 lines and maintain separation of concerns.
 */

import type {
  AuthMethod,
  AuthType,
  ProviderRegistryEntry,
} from './provider-config';

/**
 * Supported providers (those with tool calling support)
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
  'deepseek',
  'moonshot',
  'galadriel',
  'azure',
  'zai',
  'codex',
  'github-copilot',
] as const;

export type ProviderId = (typeof SUPPORTED_PROVIDERS)[number];

/**
 * Provider registry with configuration details for each provider
 */
const PROVIDER_REGISTRY: ProviderRegistryEntry[] = [
  {
    id: 'openai',
    name: 'OpenAI API',
    baseUrl: 'https://api.openai.com/v1',
    envVar: '',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: false,
    description:
      'OpenAI-compatible API for local models (vLLM, Ollama, etc.). Cloud OpenAI models use Codex credentials.',
  },
  {
    id: 'anthropic',
    name: 'Anthropic',
    baseUrl: 'https://api.anthropic.com/v1',
    envVar: 'ANTHROPIC_API_KEY',
    authMethod: 'x-api-key',
    authType: 'oauth',
    requiresApiKey: true,
    description: 'Anthropic Claude models',
  },
  {
    id: 'cohere',
    name: 'Cohere',
    baseUrl: 'https://api.cohere.ai/v1',
    envVar: 'COHERE_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Cohere language models',
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta',
    envVar: 'GOOGLE_GENERATIVE_AI_API_KEY',
    authMethod: 'query_param',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Google Gemini models',
  },
  {
    id: 'mistral',
    name: 'Mistral AI',
    baseUrl: 'https://api.mistral.ai/v1',
    envVar: 'MISTRAL_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Mistral AI models',
  },
  {
    id: 'xai',
    name: 'xAI',
    baseUrl: 'https://api.x.ai/v1',
    envVar: 'XAI_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'xAI Grok models',
  },
  {
    id: 'together',
    name: 'Together AI',
    baseUrl: 'https://api.together.xyz/v1',
    envVar: 'TOGETHER_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Together AI hosted models',
  },
  {
    id: 'huggingface',
    name: 'Hugging Face',
    baseUrl: 'https://api-inference.huggingface.co/models',
    envVar: 'HUGGINGFACE_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Hugging Face inference API',
  },
  {
    id: 'openrouter',
    name: 'OpenRouter',
    baseUrl: 'https://openrouter.ai/api/v1',
    envVar: 'OPENROUTER_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'OpenRouter unified API',
  },
  {
    id: 'groq',
    name: 'Groq',
    baseUrl: 'https://api.groq.com/openai/v1',
    envVar: 'GROQ_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Groq fast inference',
  },
  {
    id: 'deepseek',
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com/v1',
    envVar: 'DEEPSEEK_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'DeepSeek models',
  },
  {
    id: 'moonshot',
    name: 'Moonshot',
    baseUrl: 'https://api.moonshot.cn/v1',
    envVar: 'MOONSHOT_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Moonshot AI models',
  },
  {
    id: 'galadriel',
    name: 'Galadriel',
    baseUrl: 'https://api.galadriel.com/v1',
    envVar: 'GALADRIEL_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Galadriel AI models',
  },
  {
    id: 'azure',
    name: 'Azure OpenAI',
    baseUrl: '', // Requires custom endpoint
    envVar: 'AZURE_OPENAI_API_KEY',
    authMethod: 'x-api-key',
    authType: 'api-key',
    requiresApiKey: true,
    description: 'Azure OpenAI Service',
  },
  {
    id: 'zai',
    name: 'Z.AI',
    baseUrl: 'https://api.z.ai/api/paas/v4',
    envVar: 'ZAI_API_KEY',
    authMethod: 'bearer',
    authType: 'api-key',
    requiresApiKey: true,
    description:
      'Z.AI GLM models. Use ZAI_API_KEY for normal API, ZAI_PLAN_API_KEY for coding plan API (https://api.z.ai/api/coding/paas/v4)',
  },
  {
    id: 'codex',
    name: 'Codex (ChatGPT)',
    baseUrl: 'https://api.openai.com/v1',
    envVar: 'CODEX_API_KEY',
    authMethod: 'bearer',
    authType: 'oauth',
    requiresApiKey: false,
    description: 'OpenAI Codex via ChatGPT Pro/Plus OAuth',
  },
  {
    id: 'github-copilot',
    name: 'GitHub Copilot',
    baseUrl: 'https://api.githubcopilot.com',
    envVar: '',
    authMethod: 'bearer',
    authType: 'oauth',
    requiresApiKey: false,
    description:
      'GitHub Copilot via OAuth device flow (RFC 8628). Supports github.com and GitHub Enterprise deployments. Tokens are stored in ~/.fspec/credentials/copilot_auth.json and never expire.',
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
 * Check if a provider uses OAuth authentication
 */
export function isOAuthProvider(providerId: string): boolean {
  const entry = getProviderRegistryEntry(providerId);
  return entry?.authType === 'oauth';
}
