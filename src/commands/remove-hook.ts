/**
 * Remove hook command
 */

import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
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
      h => h.name !== options.name
    );
  }

  // Write updated config
  await writeFile(configPath, JSON.stringify(config, null, 2));
}

export function registerRemoveHookCommand(program: Command): void {
  program
    .command('remove-hook')
    .description('Remove a lifecycle hook from the configuration')
    .argument(
      '<event>',
      'Event name of the hook to remove (e.g., pre-update-work-unit-status, post-implementing)'
    )
    .argument(
      '<name>',
      'Name of the hook to remove (e.g., validate-feature, run-tests)'
    )
    .action(async (event: string, name: string, options: { cwd?: string }) => {
      await removeHook({ event, name, ...options });
    });
}
