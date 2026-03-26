/**
 * Feature: spec/features/foundation-event-storm-remove-commands.feature
 * Scenario: Remove aggregate from bounded context
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type { GenericFoundation } from '../types/generic-foundation';
import { generateFoundationMdCommand } from './generate-foundation-md';
import { output } from '../utils/output';

export interface RemoveAggregateFromFoundationOptions {
  cwd?: string;
}

/**
 * Remove an aggregate from a bounded context in the foundation Event Storm.
 * Uses soft-delete (sets deleted: true).
 *
 * @param contextName - The bounded context name
 * @param aggregateName - The aggregate name to remove
 * @param options - Command options
 * @returns Result with success status
 */
export async function removeAggregateFromFoundation(
  contextName: string,
  aggregateName: string,
  options: RemoveAggregateFromFoundationOptions = {}
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

      // Find the aggregate within this context
      const aggregate = data.eventStorm.items.find(
        item =>
          item.type === 'aggregate' &&
          item.text === aggregateName &&
          !item.deleted &&
          'boundedContextId' in item &&
          (item as Record<string, unknown>).boundedContextId ===
            boundedContext.id
      );

      if (!aggregate) {
        throw new Error(
          `Aggregate '${aggregateName}' not found in bounded context '${contextName}'`
        );
      }

      aggregate.deleted = true;
    }
  );

  await generateFoundationMdCommand({ cwd });

  return {
    success: true,
    message: `Removed aggregate "${aggregateName}" from "${contextName}" bounded context`,
  };
}

/**
 * CLI command wrapper
 */
export async function removeAggregateFromFoundationCommand(
  contextName: string,
  aggregateName: string,
  options: RemoveAggregateFromFoundationOptions
): Promise<void> {
  try {
    const result = await removeAggregateFromFoundation(
      contextName,
      aggregateName,
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
 * Register remove-aggregate-from-foundation command with Commander
 */
export function registerRemoveAggregateFromFoundationCommand(
  program: Command
): void {
  program
    .command('remove-aggregate-from-foundation')
    .description(
      'Remove an aggregate from a foundation bounded context (soft-delete)'
    )
    .argument('<context-name>', 'Bounded context name')
    .argument('<aggregate-name>', 'Aggregate name to remove')
    .action(
      async (
        contextName: string,
        aggregateName: string,
        options: RemoveAggregateFromFoundationOptions
      ) => {
        await removeAggregateFromFoundationCommand(
          contextName,
          aggregateName,
          options
        );
      }
    );
}
