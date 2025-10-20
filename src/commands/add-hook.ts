/**
 * Add hook command
 */

import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
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

export function registerAddHookCommand(program: Command): void {
  program
    .command('add-hook')
    .description('Add a lifecycle hook to the configuration')
    .argument(
      '<event>',
      'Event name when hook should run (e.g., pre-update-work-unit-status, post-implementing)'
    )
    .argument(
      '<name>',
      'Unique name for this hook (e.g., validate-feature, run-tests)'
    )
    .requiredOption(
      '--command <path>',
      'Path to hook script, relative to project root (e.g., spec/hooks/validate.sh)'
    )
    .option(
      '--blocking',
      'If set, hook failure prevents command execution (pre-hooks) or sets exit code to 1 (post-hooks)',
      false
    )
    .option(
      '--timeout <seconds>',
      'Timeout in seconds (default: 60). Hook is killed if it exceeds this time.',
      (value: string) => parseInt(value, 10)
    )
    .action(
      async (
        event: string,
        name: string,
        options: {
          command: string;
          blocking: boolean;
          timeout?: number;
          cwd?: string;
        }
      ) => {
        await addHook({ event, name, ...options });
      }
    );
}
