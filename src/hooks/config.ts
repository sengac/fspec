/**
 * Hook configuration loading and validation
 */

import { readFile, access } from 'node:fs/promises';
import { join } from 'node:path';
import type { HookConfig, HookDefinition } from './types.js';

const DEFAULT_TIMEOUT = 60;

export async function loadHookConfig(projectRoot: string): Promise<HookConfig> {
  const configPath = join(projectRoot, 'spec/fspec-hooks.json');

  let rawConfig: string;
  try {
    rawConfig = await readFile(configPath, 'utf-8');
  } catch (error: unknown) {
    throw new Error(`Failed to read hook configuration at ${configPath}`);
  }

  let config: HookConfig;
  try {
    config = JSON.parse(rawConfig);
  } catch (error: unknown) {
    throw new Error(`Invalid JSON in fspec-hooks.json: ${error instanceof Error ? error.message : String(error)}`);
  }

  // Validate hook command files exist
  for (const [event, hooks] of Object.entries(config.hooks)) {
    for (const hook of hooks) {
      const commandPath = join(projectRoot, hook.command);
      try {
        await access(commandPath);
      } catch {
        throw new Error(`Hook command not found: ${hook.command}`);
      }
    }
  }

  // Apply global defaults to hooks
  const globalTimeout = config.global?.timeout ?? DEFAULT_TIMEOUT;

  for (const [event, hooks] of Object.entries(config.hooks)) {
    for (const hook of hooks) {
      // Apply default timeout if not specified
      if (hook.timeout === undefined) {
        hook.timeout = globalTimeout;
      }
      // Apply default blocking if not specified
      if (hook.blocking === undefined) {
        hook.blocking = false;
      }
    }
  }

  return config;
}
