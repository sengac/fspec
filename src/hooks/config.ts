/**
 * Hook configuration loading and validation
 */

import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import type { HookConfig, HookDefinition } from './types.js';
import { isShellCommand, validateScriptExists } from './command-utils.js';

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
    throw new Error(
      `Invalid JSON in fspec-hooks.json: ${error instanceof Error ? error.message : String(error)}`
    );
  }

  // Validate hook command files exist (skip validation for shell commands)
  for (const [event, hooks] of Object.entries(config.hooks)) {
    for (const hook of hooks) {
      // Check if this is a shell command or script path
      const isShell = await isShellCommand(hook.command, projectRoot);

      // Only validate script paths, skip shell commands
      if (!isShell) {
        await validateScriptExists(hook.command, projectRoot);
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
