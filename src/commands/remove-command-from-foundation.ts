/**
 * Feature: spec/features/foundation-event-storm-remove-commands.feature
 * Scenario: Remove command from bounded context
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type { GenericFoundation } from '../types/generic-foundation';
import { generateFoundationMdCommand } from './generate-foundation-md';
import { output } from '../utils/output';

export interface RemoveCommandFromFoundationOptions {
  cwd?: string;
}

/**
 * Remove a command from a bounded context in the foundation Event Storm.
 * Uses soft-delete (sets deleted: true).
 *
 * @param contextName - The bounded context name
 * @param commandName - The command name to remove
 * @param options - Command options
 * @returns Result with success status
 */
export async function removeCommandFromFoundation(
  contextName: string,
  commandName: string,
  options: RemoveCommandFromFoundationOptions = {}
): Promise<{ success: boolean; message?: string }> {
  const cwd = options.cwd || process.cwd();
  const foundationPath = `${cwd}/spec/foundation.json`;

  await fileManager.readJSON<GenericFoundation>(foundationPath, {
    version: '2.0.0',
    project: { name: '', vision: '', projectType: 'other' as const },
    problemSpace: {
      primaryProblem: {
        title: '',
        description: '',
        impact: 'medium' as const,
      },
    },
    solutionSpace: { overview: '', capabilities: [] },
  });

  await fileManager.transaction<GenericFoundation>(
    foundationPath,
    async data => {
      if (!data.eventStorm) {
        throw new Error(
          `Bounded context '${contextName}' not found (no Event Storm data)`
        );
      }

      // Find bounded context
      const boundedContext = data.eventStorm.items.find(
        item =>
          item.type === 'bounded_context' &&
          item.text === contextName &&
          !item.deleted
      );

      if (!boundedContext) {
        throw new Error(`Bounded context '${contextName}' not found`);
      }

      // Find the command within this context
      const foundCommand = data.eventStorm.items.find(
        item =>
          item.type === 'command' &&
          item.text === commandName &&
          !item.deleted &&
          'boundedContextId' in item &&
          (item as Record<string, unknown>).boundedContextId ===
            boundedContext.id
      );

      if (!foundCommand) {
        throw new Error(
          `Command '${commandName}' not found in bounded context '${contextName}'`
        );
      }

      foundCommand.deleted = true;
    }
  );

  await generateFoundationMdCommand({ cwd });

  return {
    success: true,
    message: `Removed command "${commandName}" from "${contextName}" bounded context`,
  };
}

/**
 * CLI command wrapper
 */
export async function removeCommandFromFoundationCommand(
  contextName: string,
  commandName: string,
  options: RemoveCommandFromFoundationOptions
): Promise<void> {
  try {
    const result = await removeCommandFromFoundation(
      contextName,
      commandName,
      options
    );

    if (!result.success) {
      output.error('Error:', result.message);
      process.exit(1);
    }

    output.log('✓', result.message);
    process.exit(0);
  } catch (error: unknown) {
    output.error(
      chalk.red('Error:'),
      error instanceof Error ? error.message : 'Unknown error'
    );
    process.exit(1);
  }
}

/**
 * Register remove-command-from-foundation command with Commander
 */
export function registerRemoveCommandFromFoundationCommand(
  program: Command
): void {
  program
    .command('remove-command-from-foundation')
    .description(
      'Remove a command from a foundation bounded context (soft-delete)'
    )
    .argument('<context-name>', 'Bounded context name')
    .argument('<command-name>', 'Command name to remove')
    .action(
      async (
        contextName: string,
        commandName: string,
        options: RemoveCommandFromFoundationOptions
      ) => {
        await removeCommandFromFoundationCommand(
          contextName,
          commandName,
          options
        );
      }
    );
}
