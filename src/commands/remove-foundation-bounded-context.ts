/**
 * Feature: spec/features/foundation-event-storm-remove-commands.feature
 * Scenario: Remove an empty bounded context
 * Scenario: Refuse to remove non-empty bounded context without cascade flag
 * Scenario: Remove non-empty bounded context with cascade flag
 * Scenario: FOUNDATION.md regenerated after removal
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type { GenericFoundation } from '../types/generic-foundation';
import { generateFoundationMdCommand } from './generate-foundation-md';
import { output } from '../utils/output';

export interface RemoveFoundationBoundedContextOptions {
  cwd?: string;
  cascade?: boolean;
}

/**
 * Remove a bounded context from the foundation-level Big Picture Event Storm.
 * Uses soft-delete (sets deleted: true) consistent with ItemWithId pattern.
 *
 * @param contextName - The bounded context name to remove
 * @param options - Command options (cwd, cascade)
 * @returns Result with success status
 */
export async function removeFoundationBoundedContext(
  contextName: string,
  options: RemoveFoundationBoundedContextOptions = {}
): Promise<{ success: boolean; message?: string }> {
  const cwd = options.cwd || process.cwd();
  const foundationPath = `${cwd}/spec/foundation.json`;

  // Read and validate foundation exists
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

  // Use transaction for atomic update
  await fileManager.transaction<GenericFoundation>(
    foundationPath,
    async data => {
      if (!data.eventStorm) {
        throw new Error(
          `Bounded context '${contextName}' not found (no Event Storm data)`
        );
      }

      // Find the bounded context by name (non-deleted)
      const boundedContext = data.eventStorm.items.find(
        item =>
          item.type === 'bounded_context' &&
          item.text === contextName &&
          !item.deleted
      );

      if (!boundedContext) {
        throw new Error(`Bounded context '${contextName}' not found`);
      }

      // Count active child items
      const childItems = data.eventStorm.items.filter(
        item =>
          !item.deleted &&
          'boundedContextId' in item &&
          (item as Record<string, unknown>).boundedContextId ===
            boundedContext.id
      );

      // If has children and no cascade flag, refuse
      if (childItems.length > 0 && !options.cascade) {
        throw new Error(
          `Bounded context '${contextName}' has ${childItems.length} child items. ` +
            `Use --cascade to remove the context and all its children.`
        );
      }

      // Soft-delete the bounded context
      boundedContext.deleted = true;

      // If cascade, soft-delete all children
      if (options.cascade && childItems.length > 0) {
        for (const child of childItems) {
          child.deleted = true;
        }
      }
    }
  );

  // Auto-regenerate FOUNDATION.md
  await generateFoundationMdCommand({ cwd });

  const cascadeMsg = options.cascade ? ' and all its children' : '';
  return {
    success: true,
    message: `Removed bounded context "${contextName}"${cascadeMsg} from foundation Event Storm`,
  };
}

/**
 * CLI command wrapper for remove-foundation-bounded-context
 */
export async function removeFoundationBoundedContextCommand(
  contextName: string,
  options: RemoveFoundationBoundedContextOptions
): Promise<void> {
  try {
    const result = await removeFoundationBoundedContext(contextName, options);

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
 * Register remove-foundation-bounded-context command with Commander
 */
export function registerRemoveFoundationBoundedContextCommand(
  program: Command
): void {
  program
    .command('remove-foundation-bounded-context')
    .description(
      'Remove a bounded context from foundation Big Picture Event Storm (soft-delete)'
    )
    .argument('<context-name>', 'Bounded context name to remove')
    .option(
      '--cascade',
      'Also remove all child items (aggregates, events, commands)'
    )
    .action(
      async (
        contextName: string,
        options: RemoveFoundationBoundedContextOptions
      ) => {
        await removeFoundationBoundedContextCommand(contextName, options);
      }
    );
}
