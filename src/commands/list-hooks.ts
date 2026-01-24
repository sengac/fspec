/**
 * List hooks command
 */

import { readFile } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import type { HookConfig } from '../hooks/types';

export interface ListHooksOptions {
  cwd?: string;
}

export interface ListHooksResult {
  events: Array<{
    event: string;
    hooks: string[];
  }>;
  message?: string;
}

export async function listHooks(
  options: ListHooksOptions = {}
): Promise<ListHooksResult> {
  const cwd = options.cwd ?? process.cwd();
  const configPath = join(cwd, 'spec', 'fspec-hooks.json');

  try {
    const configContent = await readFile(configPath, 'utf-8');
    const config = JSON.parse(configContent) as HookConfig;

    const events = Object.entries(config.hooks).map(([event, hooks]) => ({
      event,
      hooks: hooks.map(h => h.name),
    }));

    return { events };
  } catch (error: unknown) {
    // Config file doesn't exist
    return {
      events: [],
      message: 'No hooks are configured',
    };
  }
}

export function registerListHooksCommand(program: Command): void {
  program
    .command('list-hooks')
    .description('List all configured lifecycle hooks')
    .action(async (options: { cwd?: string }) => {
      await listHooks(options);
    });
}
