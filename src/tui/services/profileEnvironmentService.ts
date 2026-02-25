/**
 * Profile Environment Service
 *
 * PROV-008: Isolates environment variable configuration for profile-based models.
 *
 * This service sets the required OPENAI_* environment variables before
 * any Rust session operations, ensuring the LLM client can connect to
 * the correct local server endpoint.
 *
 * Environment Variables Set:
 * - OPENAI_BASE_URL: The server endpoint URL
 * - OPENAI_API_KEY: Authentication key for the server
 * - OPENAI_CONTEXT_WINDOW: Optional context window size
 * - OPENAI_MAX_OUTPUT_TOKENS: Optional max output tokens limit
 */

import type { ProfileConfig } from '../../utils/provider-config';

/**
 * Configure environment variables for profile-based models.
 *
 * Called before any Rust session operations to ensure
 * OPENAI_BASE_URL and OPENAI_API_KEY are set correctly
 * for local LLM servers (vLLM, Ollama, etc.)
 *
 * @param config - Profile configuration with baseUrl and apiKey
 */
export function configureProfileEnvironment(config: ProfileConfig): void {
  process.env.OPENAI_BASE_URL = config.baseUrl;
  process.env.OPENAI_API_KEY = config.apiKey;

  if (config.contextWindow) {
    process.env.OPENAI_CONTEXT_WINDOW = String(config.contextWindow);
  }
  if (config.maxOutputTokens) {
    process.env.OPENAI_MAX_OUTPUT_TOKENS = String(config.maxOutputTokens);
  }
}
