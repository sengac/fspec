/**
 * Remove hook command
 */

import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { HookConfig } from '../hooks/types.js';

export interface RemoveHookOptions {
  name: string;
  event: string;
  cwd?: string;
}

export async function removeHook(options: RemoveHookOptions): Promise<void> {
  const cwd = options.cwd ?? process.cwd();
  const configPath = join(cwd, 'spec', 'fspec-hooks.json');

  const configContent = await readFile(configPath, 'utf-8');
  const config = JSON.parse(configContent) as HookConfig;

  // Remove hook from event
  if (config.hooks[options.event]) {
    config.hooks[options.event] = config.hooks[options.event].filter(
      (h) => h.name !== options.name
    );
  }

  // Write updated config
  await writeFile(configPath, JSON.stringify(config, null, 2));
}
