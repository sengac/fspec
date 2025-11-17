// Feature: spec/features/big-picture-event-storm-in-foundation-json.feature

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type {
  GenericFoundation,
  FoundationEventStorm,
} from '../types/generic-foundation';
import type { EventStormBoundedContext } from '../types';
import { generateFoundationMdCommand } from './generate-foundation-md';

export interface AddFoundationBoundedContextOptions {
  cwd?: string;
}

/**
 * Add a bounded context to the foundation-level Big Picture Event Storm
 *
 * @param text - The bounded context name/description
 * @param options - Command options
 * @returns Result with success status
 */
export async function addFoundationBoundedContext(
  text: string,
  options: AddFoundationBoundedContextOptions = {}
): Promise<{ success: boolean; message?: string }> {
  const cwd = options.cwd || process.cwd();
  const foundationPath = `${cwd}/spec/foundation.json`;

  // Read foundation.json with defaults
  const foundation = await fileManager.readJSON<GenericFoundation>(
    foundationPath,
    {
      version: '2.0.0',
      project: {
        name: '',
        vision: '',
        projectType: 'other' as const,
      },
      problemSpace: {
        primaryProblem: {
          title: '',
          description: '',
          impact: 'medium' as const,
        },
      },
      solutionSpace: {
        overview: '',
        capabilities: [],
      },
    }
  );

  // Use transaction for atomic update
  await fileManager.transaction<GenericFoundation>(
    foundationPath,
    async data => {
      // Initialize eventStorm section if missing
      if (!data.eventStorm) {
        data.eventStorm = {
          level: 'big_picture',
          items: [],
          nextItemId: 1,
        };
      }

      // Create bounded context item
      const boundedContext: EventStormBoundedContext = {
        id: data.eventStorm.nextItemId,
        type: 'bounded_context',
        text,
        color: null,
        deleted: false,
        createdAt: new Date().toISOString(),
      };

      // Add item and increment counter
      data.eventStorm.items.push(boundedContext);
      data.eventStorm.nextItemId++;
    }
  );

  // Auto-regenerate FOUNDATION.md after updating foundation.json
  await generateFoundationMdCommand({ cwd });

  return {
    success: true,
    message: `Added bounded context "${text}" to foundation Event Storm`,
  };
}

/**
 * CLI command wrapper for add-foundation-bounded-context
 */
export async function addFoundationBoundedContextCommand(
  text: string
): Promise<void> {
  try {
    const result = await addFoundationBoundedContext(text);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.message);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);
    process.exit(0);
  } catch (error: unknown) {
    console.error(
      chalk.red('Error:'),
      error instanceof Error ? error.message : 'Unknown error'
    );
    process.exit(1);
  }
}

/**
 * Register add-foundation-bounded-context command with Commander
 */
export function registerAddFoundationBoundedContextCommand(
  program: Command
): void {
  program
    .command('add-foundation-bounded-context')
    .description('Add a bounded context to foundation Big Picture Event Storm')
    .argument('<text>', 'Bounded context name or description')
    .action(async (text: string) => {
      await addFoundationBoundedContextCommand(text);
    });
}
