/**
 * Add hook command
 */

import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import type { HookConfig } from '../hooks/types.js';

export interface AddHookOptions {
  name: string;
  event: string;
  command: string;
  blocking: boolean;
  timeout?: number;
  cwd?: string;
}

export async function addHook(options: AddHookOptions): Promise<void> {
  const cwd = options.cwd ?? process.cwd();
  const configPath = join(cwd, 'spec', 'fspec-hooks.json');

  let config: HookConfig;

  try {
    const configContent = await readFile(configPath, 'utf-8');
    config = JSON.parse(configContent) as HookConfig;
  } catch {
    // Config doesn't exist, create new one
    config = { hooks: {} };
  }

  // Initialize event array if it doesn't exist
  if (!config.hooks[options.event]) {
    config.hooks[options.event] = [];
  }

  // Add hook to event
  config.hooks[options.event].push({
    name: options.name,
    command: options.command,
    blocking: options.blocking,
    timeout: options.timeout,
  });

  // Ensure spec directory exists
  await mkdir(join(cwd, 'spec'), { recursive: true });

  // Write updated config
  await writeFile(configPath, JSON.stringify(config, null, 2));
}
