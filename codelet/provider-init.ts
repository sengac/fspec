/**
 * Provider initialization with FspecTool callback registration
 *
 * This module handles initializing the FspecTool callback for each provider
 * before creating rig agents. It bridges TypeScript callback execution with
 * Rust FspecTool instances.
 */

import { initializeFspecCallback } from '../codelet/napi';
import { createFspecCallback } from '../codelet/fspec-callback';

/**
 * Initialize FspecTool callback for all providers
 *
 * This must be called during startup before creating any rig agents.
 * It registers the TypeScript callback that executes fspec commands
 * for each provider.
 */
export async function initializeFspecCallbacks(): Promise<void> {
  const callback = createFspecCallback();

  // Register the same callback for all providers
  // Each provider will use this callback to execute fspec commands
  await Promise.all([
    initializeFspecCallback('claude', callback),
    initializeFspecCallback('gemini', callback),
    initializeFspecCallback('openai', callback),
    initializeFspecCallback('zai', callback),
  ]);
}

/**
 * Initialize FspecTool callback for a specific provider
 *
 * @param provider - Provider name ('claude', 'gemini', 'openai', 'zai')
 */
export async function initializeFspecCallbackForProvider(
  provider: string
): Promise<void> {
  const callback = createFspecCallback();
  await initializeFspecCallback(provider, callback);
}
