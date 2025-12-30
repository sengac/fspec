/**
 * Credentials management for provider API keys
 *
 * Credentials are stored in ~/.fspec/credentials/credentials.json
 * with secure file permissions (600 for file, 700 for directory).
 *
 * Resolution priority chain:
 * 1. Explicit credentials (passed programmatically)
 * 2. Credentials file (~/.fspec/credentials/credentials.json)
 * 3. Environment variables
 * 4. .env file (lowest priority fallback)
 */

import { readFile, writeFile, mkdir, chmod, stat } from 'fs/promises';
import { join, dirname } from 'path';
import { existsSync } from 'fs';
import { parse as dotenvParse } from 'dotenv';
import { readFileSync } from 'fs';
import { getFspecUserDir } from './config.js';

/**
 * Provider credential data
 */
export interface ProviderCredential {
  apiKey: string;
  lastUpdated: string;
}

/**
 * Credentials file structure
 */
export interface CredentialsFile {
  version: number;
  providers: Record<string, ProviderCredential>;
}

/**
 * Provider config returned by getProviderConfig
 */
export interface ProviderConfigResult {
  apiKey?: string;
  source?: 'explicit' | 'file' | 'env' | 'dotenv';
}

/**
 * Map of provider IDs to their environment variable names
 * Some providers support multiple env vars (checked in order)
 */
const PROVIDER_ENV_VARS: Record<string, string[]> = {
  anthropic: ['ANTHROPIC_API_KEY', 'CLAUDE_CODE_OAUTH_TOKEN'],
  openai: ['OPENAI_API_KEY'],
  cohere: ['COHERE_API_KEY'],
  gemini: ['GOOGLE_GENERATIVE_AI_API_KEY', 'GEMINI_API_KEY'],
  mistral: ['MISTRAL_API_KEY'],
  xai: ['XAI_API_KEY'],
  together: ['TOGETHER_API_KEY'],
  huggingface: ['HUGGINGFACE_API_KEY', 'HF_TOKEN'],
  openrouter: ['OPENROUTER_API_KEY'],
  groq: ['GROQ_API_KEY'],
  ollama: ['OLLAMA_API_KEY'], // Optional for Ollama
  deepseek: ['DEEPSEEK_API_KEY'],
  perplexity: ['PERPLEXITY_API_KEY'],
  moonshot: ['MOONSHOT_API_KEY'],
  hyperbolic: ['HYPERBOLIC_API_KEY'],
  mira: ['MIRA_API_KEY'],
  galadriel: ['GALADRIEL_API_KEY'],
  azure: ['AZURE_OPENAI_API_KEY'],
  voyageai: ['VOYAGEAI_API_KEY'],
};

/**
 * Get the path to the credentials file
 */
export function getCredentialsPath(): string {
  return join(getFspecUserDir(), 'credentials', 'credentials.json');
}

/**
 * Get the directory for credentials
 */
function getCredentialsDir(): string {
  return join(getFspecUserDir(), 'credentials');
}

/**
 * Load credentials from file
 */
export async function loadCredentials(): Promise<CredentialsFile> {
  const credPath = getCredentialsPath();

  try {
    const content = await readFile(credPath, 'utf-8');
    if (!content.trim()) {
      return { version: 1, providers: {} };
    }
    return JSON.parse(content);
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return { version: 1, providers: {} };
    }
    throw error;
  }
}

/**
 * Save credential for a provider with secure file permissions
 *
 * Creates credentials directory with 700 permissions
 * Creates/updates credentials file with 600 permissions
 *
 * NEVER logs the API key - only masked version if needed
 */
export async function saveCredential(
  providerId: string,
  apiKey: string
): Promise<void> {
  const credDir = getCredentialsDir();
  const credPath = getCredentialsPath();

  // Create credentials directory with 700 permissions
  await mkdir(credDir, { recursive: true });
  await chmod(credDir, 0o700);

  // Load existing credentials
  const credentials = await loadCredentials();

  // Update provider credential
  credentials.providers[providerId] = {
    apiKey,
    lastUpdated: new Date().toISOString(),
  };

  // Write credentials file
  await writeFile(credPath, JSON.stringify(credentials, null, 2), 'utf-8');

  // Set file permissions to 600 (owner read/write only)
  await chmod(credPath, 0o600);
}

/**
 * Delete credential for a provider
 */
export async function deleteCredential(providerId: string): Promise<void> {
  const credPath = getCredentialsPath();

  // Load existing credentials
  const credentials = await loadCredentials();

  // Remove provider credential
  delete credentials.providers[providerId];

  // Write updated credentials file
  await writeFile(credPath, JSON.stringify(credentials, null, 2), 'utf-8');

  // Maintain secure permissions
  await chmod(credPath, 0o600);
}

/**
 * Get provider configuration including resolved API key
 *
 * Resolution priority:
 * 1. Credentials file (~/.fspec/credentials/credentials.json)
 * 2. Environment variable (already in process.env)
 * 3. .env file (loaded from current working directory)
 */
export async function getProviderConfig(
  providerId: string
): Promise<ProviderConfigResult> {
  // Try credentials file first
  const credentials = await loadCredentials();
  const providerCred = credentials.providers[providerId];

  if (providerCred?.apiKey) {
    return {
      apiKey: providerCred.apiKey,
      source: 'file',
    };
  }

  // Try environment variables (check all possible env vars for this provider)
  const envVars = PROVIDER_ENV_VARS[providerId];
  if (envVars) {
    for (const envVar of envVars) {
      if (process.env[envVar]) {
        return {
          apiKey: process.env[envVar],
          source: 'env',
        };
      }
    }
  }

  // Try .env file from current working directory (Rust loads this but it doesn't propagate to TypeScript)
  const envPath = join(process.cwd(), '.env');
  if (existsSync(envPath) && envVars) {
    // Parse .env file silently (dotenvParse doesn't output anything to console)
    const envContent = readFileSync(envPath, 'utf-8');
    const parsed = dotenvParse(envContent);

    for (const envVar of envVars) {
      if (parsed[envVar]) {
        return {
          apiKey: parsed[envVar],
          source: 'dotenv',
        };
      }
    }
  }

  return {};
}

/**
 * Resolve credential for a provider following the priority chain
 *
 * Priority:
 * 1. Explicit credentials (passed as parameter)
 * 2. Credentials file
 * 3. Environment variables
 * 4. .env file (if dotenvDir provided)
 *
 * @param providerId - The provider ID (e.g., 'anthropic', 'openai')
 * @param explicitKey - Optional explicit API key (highest priority)
 * @param dotenvDir - Optional directory containing .env file
 */
export async function resolveCredential(
  providerId: string,
  explicitKey?: string,
  dotenvDir?: string
): Promise<string | undefined> {
  // 1. Explicit credentials take highest priority
  if (explicitKey) {
    return explicitKey;
  }

  // 2. Try credentials file
  const credentials = await loadCredentials();
  const providerCred = credentials.providers[providerId];

  if (providerCred?.apiKey) {
    return providerCred.apiKey;
  }

  // 3. Try environment variables (check all possible env vars for this provider)
  const envVars = PROVIDER_ENV_VARS[providerId];
  if (envVars) {
    for (const envVar of envVars) {
      if (process.env[envVar]) {
        return process.env[envVar];
      }
    }
  }

  // 4. Try .env file (lowest priority)
  if (dotenvDir && envVars) {
    const envPath = join(dotenvDir, '.env');
    if (existsSync(envPath)) {
      // Parse .env file silently (dotenvParse doesn't output anything to console)
      const envContent = readFileSync(envPath, 'utf-8');
      const parsed = dotenvParse(envContent);

      for (const envVar of envVars) {
        if (parsed[envVar]) {
          return parsed[envVar];
        }
      }
    }
  }

  return undefined;
}

/**
 * Mask API key for display
 *
 * Shows prefix and last 4 characters, masks middle with dots
 * Example: "sk-ant-api03-abcdefghijklmnop" -> "sk-ant-••••••••mnop"
 *
 * NEVER log the full key - only use this masked version
 */
export function maskApiKey(apiKey: string): string {
  if (!apiKey || apiKey.length < 12) {
    return '••••••••';
  }

  // Find prefix (characters up to first dash after common prefixes)
  const prefixMatch = apiKey.match(/^(sk-ant-|sk-|gsk_|AIza|xai-)/);
  const prefix = prefixMatch ? prefixMatch[0] : apiKey.slice(0, 6);

  // Last 4 characters
  const suffix = apiKey.slice(-4);

  // Mask middle
  return `${prefix}••••••••${suffix}`;
}

/**
 * Get primary environment variable name for a provider
 * Returns the first (primary) env var for the provider
 */
export function getProviderEnvVar(providerId: string): string | undefined {
  const envVars = PROVIDER_ENV_VARS[providerId];
  return envVars?.[0];
}

/**
 * Get all environment variable names for a provider
 * Some providers support multiple env vars (checked in order)
 */
export function getProviderEnvVars(providerId: string): string[] | undefined {
  return PROVIDER_ENV_VARS[providerId];
}
