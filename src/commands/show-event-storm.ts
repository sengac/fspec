/**
 * Show Event Storm command
 * Outputs Event Storm artifacts as JSON data for AI analysis
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import { fileManager } from '../utils/file-manager';
import type { WorkUnitsData, EventStormItem } from '../types';

export interface ShowEventStormOptions {
  workUnitId: string;
  cwd?: string;
}

export interface ShowEventStormResult {
  success: boolean;
  data?: EventStormItem[];
  error?: string;
}

/**
 * Show Event Storm data as JSON
 * Returns raw structural data without semantic interpretation
 *
 * @param options - Command options
 * @returns Event Storm items array (filtered for deleted=false)
 */
export async function showEventStorm(
  options: ShowEventStormOptions
): Promise<ShowEventStormResult> {
  const { workUnitId, cwd = process.cwd() } = options;
  const workUnitsFile = `${cwd}/spec/work-units.json`;

  try {
    // Read work units data
    const workUnitsData = await fileManager.readJSON<WorkUnitsData>(
      workUnitsFile,
      {
        version: '1.0.0',
        workUnits: {},
        states: {},
      }
    );

    // Get work unit
    const workUnit = workUnitsData.workUnits[workUnitId];
    if (!workUnit) {
      return {
        success: false,
        error: `Work unit ${workUnitId} not found`,
      };
    }

    // Check for Event Storm data
    if (!workUnit.eventStorm || !workUnit.eventStorm.items) {
      return {
        success: false,
        error: `Work unit ${workUnitId} has no Event Storm data`,
      };
    }

    // Filter out deleted items
    const activeItems = workUnit.eventStorm.items.filter(item => !item.deleted);

    return {
      success: true,
      data: activeItems,
    };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error occurred',
    };
  }
}

/**
 * CLI command wrapper for show-event-storm
 */
export async function showEventStormCommand(workUnitId: string): Promise<void> {
  try {
    const result = await showEventStorm({ workUnitId });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
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
 * Register show-event-storm command with Commander
 */
export function registerShowEventStormCommand(program: Command): void {
  program
    .command('show-event-storm')
    .description(
      'Display Event Storm artifacts as JSON (no semantic interpretation)'
    )
    .argument('<work-unit-id>', 'Work unit ID to query')
    .action(async (workUnitId: string) => {
      await showEventStormCommand(workUnitId);
    });
}
