// Feature: spec/features/big-picture-event-storm-in-foundation-json.feature

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type { GenericFoundation } from '../types/generic-foundation';
import type { EventStormItem } from '../types';

export interface ShowFoundationEventStormOptions {
  type?: string;
  cwd?: string;
}

/**
 * Show foundation-level Event Storm items (filtered by type if specified)
 *
 * @param options - Command options
 * @returns Result with success status and data
 */
export async function showFoundationEventStorm(
  options: ShowFoundationEventStormOptions = {}
): Promise<{ success: boolean; data: EventStormItem[]; message?: string }> {
  const cwd = options.cwd || process.cwd();
  const foundationPath = `${cwd}/spec/foundation.json`;

  // Read foundation.json
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

  // Check if eventStorm exists
  if (!foundation.eventStorm) {
    return {
      success: true,
      data: [],
      message: 'No Event Storm data in foundation.json',
    };
  }

  // Filter out deleted items (structural filtering only)
  let items = foundation.eventStorm.items.filter(item => !item.deleted);

  // Filter by type if specified (structural filtering only)
  if (options.type) {
    items = items.filter(item => item.type === options.type);
  }

  return {
    success: true,
    data: items,
  };
}

/**
 * CLI command wrapper for show-foundation-event-storm
 */
export async function showFoundationEventStormCommand(options: {
  type?: string;
}): Promise<void> {
  try {
    const result = await showFoundationEventStorm(options);

    if (!result.success) {
      console.error(chalk.red('Error:'), result.message);
      process.exit(1);
    }

    // Output JSON to stdout
    console.log(JSON.stringify(result.data, null, 2));
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
 * Register show-foundation-event-storm command with Commander
 */
export function registerShowFoundationEventStormCommand(
  program: Command
): void {
  program
    .command('show-foundation-event-storm')
    .description(
      'Display foundation Event Storm artifacts as JSON (no semantic interpretation)'
    )
    .option('--type <type>', 'Filter by Event Storm item type')
    .action(async (options: { type?: string }) => {
      await showFoundationEventStormCommand(options);
    });
}
