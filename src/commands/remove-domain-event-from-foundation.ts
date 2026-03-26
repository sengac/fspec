/**
 * Feature: spec/features/foundation-event-storm-remove-commands.feature
 * Scenario: Remove domain event from bounded context
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type { GenericFoundation } from '../types/generic-foundation';
import { generateFoundationMdCommand } from './generate-foundation-md';
import { output } from '../utils/output';

export interface RemoveDomainEventFromFoundationOptions {
  cwd?: string;
}

/**
 * Remove a domain event from a bounded context in the foundation Event Storm.
 * Uses soft-delete (sets deleted: true).
 *
 * @param contextName - The bounded context name
 * @param eventName - The domain event name to remove
 * @param options - Command options
 * @returns Result with success status
 */
export async function removeDomainEventFromFoundation(
  contextName: string,
  eventName: string,
  options: RemoveDomainEventFromFoundationOptions = {}
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

      // Find the domain event within this context
      const domainEvent = data.eventStorm.items.find(
        item =>
          item.type === 'event' &&
          item.text === eventName &&
          !item.deleted &&
          'boundedContextId' in item &&
          (item as Record<string, unknown>).boundedContextId ===
            boundedContext.id
      );

      if (!domainEvent) {
        throw new Error(
          `Domain event '${eventName}' not found in bounded context '${contextName}'`
        );
      }

      domainEvent.deleted = true;
    }
  );

  await generateFoundationMdCommand({ cwd });

  return {
    success: true,
    message: `Removed domain event "${eventName}" from "${contextName}" bounded context`,
  };
}

/**
 * CLI command wrapper
 */
export async function removeDomainEventFromFoundationCommand(
  contextName: string,
  eventName: string,
  options: RemoveDomainEventFromFoundationOptions
): Promise<void> {
  try {
    const result = await removeDomainEventFromFoundation(
      contextName,
      eventName,
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
 * Register remove-domain-event-from-foundation command with Commander
 */
export function registerRemoveDomainEventFromFoundationCommand(
  program: Command
): void {
  program
    .command('remove-domain-event-from-foundation')
    .description(
      'Remove a domain event from a foundation bounded context (soft-delete)'
    )
    .argument('<context-name>', 'Bounded context name')
    .argument('<event-name>', 'Domain event name to remove')
    .action(
      async (
        contextName: string,
        eventName: string,
        options: RemoveDomainEventFromFoundationOptions
      ) => {
        await removeDomainEventFromFoundationCommand(
          contextName,
          eventName,
          options
        );
      }
    );
}
