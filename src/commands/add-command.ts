/**
 * Add command to Event Storm section of work unit
 * Coverage: EXMAP-006
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormCommand } from '../types';
import { fileManager } from '../utils/file-manager';

export interface AddCommandOptions {
  workUnitId: string;
  text: string;
  actor?: string;
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddCommandResult {
  success: boolean;
  error?: string;
  commandId?: number;
}

/**
 * Add command to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and command ID
 */
export async function addCommand(
  options: AddCommandOptions
): Promise<AddCommandResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  // Check if work-units.json exists
  if (!existsSync(workUnitsFile)) {
    return {
      success: false,
      error: 'spec/work-units.json not found. Run fspec init first.',
    };
  }

  try {
    // Read work units data
    const workUnitsData = await fileManager.readJSON<WorkUnitsData>(
      workUnitsFile,
      {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        states: {
          backlog: [],
          specifying: [],
          testing: [],
          implementing: [],
          validating: [],
          done: [],
          blocked: [],
        },
        workUnits: {},
      }
    );

    // Validate work unit exists
    const workUnit = workUnitsData.workUnits[options.workUnitId];
    if (!workUnit) {
      return {
        success: false,
        error: `Work unit ${options.workUnitId} not found`,
      };
    }

    // Validate work unit is not in done/blocked state
    if (workUnit.status === 'done' || workUnit.status === 'blocked') {
      return {
        success: false,
        error: `Cannot add Event Storm items to work unit in ${workUnit.status} state`,
      };
    }

    // Initialize eventStorm section if not present
    if (!workUnit.eventStorm) {
      workUnit.eventStorm = {
        level: 'process_modeling',
        items: [],
        nextItemId: 0,
      };
    }

    // Create command item
    const commandId = workUnit.eventStorm.nextItemId;
    const command: EventStormCommand = {
      id: commandId,
      type: 'command',
      color: 'blue',
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.actor) {
      command.actor = options.actor;
    }
    if (options.timestamp !== undefined) {
      command.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      command.boundedContext = options.boundedContext;
    }

    // Add command to items array
    workUnit.eventStorm.items.push(command);
    workUnit.eventStorm.nextItemId++;

    // Update work unit timestamp
    workUnit.updatedAt = new Date().toISOString();

    // Update meta
    if (workUnitsData.meta) {
      workUnitsData.meta.lastUpdated = new Date().toISOString();
    }

    // Write updated data using transaction
    await fileManager.transaction(workUnitsFile, async data => {
      const typedData = data as WorkUnitsData;
      typedData.workUnits[options.workUnitId] = workUnit;
      if (workUnitsData.meta) {
        typedData.meta = workUnitsData.meta;
      }
    });

    return {
      success: true,
      commandId,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message || 'Unknown error occurred',
    };
  }
}

export function registerAddCommandCommand(program: Command): void {
  program
    .command('add-command')
    .description('Add command to Event Storm section of work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'Command text/name')
    .option('--actor <actor>', 'Actor who executes the command')
    .option('--timestamp <ms>', 'Timeline timestamp in milliseconds', parseInt)
    .option(
      '--bounded-context <context>',
      'Bounded context for domain association'
    )
    .action(async (workUnitId: string, text: string, options: any) => {
      try {
        const result = await addCommand({
          workUnitId,
          text,
          actor: options.actor,
          timestamp: options.timestamp,
          boundedContext: options.boundedContext,
        });

        if (!result.success) {
          console.error(chalk.red('✗ Failed to add command:'), result.error);
          process.exit(1);
        }

        console.log(
          chalk.green(
            `✓ Added command "${text}" to ${workUnitId} (ID: ${result.commandId})`
          )
        );
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add command:'), error.message);
        process.exit(1);
      }
    });
}
