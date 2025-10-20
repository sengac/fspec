/**
 * Validate hooks command
 */

import { readFile, access } from 'fs/promises';
import { join } from 'path';
import type { Command } from 'commander';
import type { HookConfig } from '../hooks/types.js';

export interface ValidateHooksOptions {
  cwd?: string;
}

export interface ValidateHooksResult {
  exitCode: number;
  valid: boolean;
  errors?: string;
}

export async function validateHooks(
  options: ValidateHooksOptions = {}
): Promise<ValidateHooksResult> {
  const cwd = options.cwd ?? process.cwd();
  const configPath = join(cwd, 'spec', 'fspec-hooks.json');

  try {
    // Read and parse config
    const configContent = await readFile(configPath, 'utf-8');
    const config = JSON.parse(configContent) as HookConfig;

    // Validate all hook scripts exist
    const errors: string[] = [];

    for (const [_event, hooks] of Object.entries(config.hooks)) {
      for (const hook of hooks) {
        const hookPath = join(cwd, hook.command);
        try {
          await access(hookPath);
        } catch {
          errors.push(`Hook command not found: ${hook.command}`);
        }
      }
    }

    if (errors.length > 0) {
      return {
        exitCode: 1,
        valid: false,
        errors: errors.join('\n'),
      };
    }

    return {
      exitCode: 0,
      valid: true,
    };
  } catch (error: unknown) {
    return {
      exitCode: 1,
      valid: false,
      errors: 'Failed to load hook configuration',
    };
  }
}

export function registerValidateHooksCommand(program: Command): void {
  program
    .command('validate-hooks')
    .description(
      'Validate hook configuration and verify that all hook scripts exist'
    )
    .action(async (options: { cwd?: string }) => {
      await validateHooks(options);
    });
}
