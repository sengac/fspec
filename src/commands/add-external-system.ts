/**
 * Add external system to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormExternalSystem } from '../types';
import { fileManager } from '../utils/file-manager';

export interface AddExternalSystemOptions {
  workUnitId: string;
  text: string;
  type?:
    | 'REST_API'
    | 'MESSAGE_QUEUE'
    | 'DATABASE'
    | 'THIRD_PARTY_SERVICE'
    | 'FILE_SYSTEM';
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddExternalSystemResult {
  success: boolean;
  error?: string;
  externalSystemId?: number;
}

/**
 * Add external system to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and external system ID
 */
export async function addExternalSystem(
  options: AddExternalSystemOptions
): Promise<AddExternalSystemResult> {
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

    // Create external system item
    const externalSystemId = workUnit.eventStorm.nextItemId;
    const externalSystem: EventStormExternalSystem = {
      id: externalSystemId,
      type: 'external_system',
      color: 'pink',
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.type) {
      externalSystem.integrationType = options.type;
    }
    if (options.timestamp !== undefined) {
      externalSystem.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      externalSystem.boundedContext = options.boundedContext;
    }

    // Add external system to items array and increment ID using transaction
    await fileManager.transaction(workUnitsFile, (data: WorkUnitsData) => {
      const wu = data.workUnits[options.workUnitId];
      if (!wu.eventStorm) {
        wu.eventStorm = {
          level: 'process_modeling',
          items: [],
          nextItemId: 0,
        };
      }
      wu.eventStorm.items.push(externalSystem);
      wu.eventStorm.nextItemId += 1;
    });

    return {
      success: true,
      externalSystemId,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: `Failed to add external system: ${errorMessage}`,
    };
  }
}

export function registerAddExternalSystemCommand(program: Command): void {
  program
    .command('add-external-system')
    .description('Add external system to Event Storm section')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'External system name')
    .option(
      '--type <type>',
      'Integration type: REST_API, MESSAGE_QUEUE, DATABASE, THIRD_PARTY_SERVICE, FILE_SYSTEM'
    )
    .option('--timestamp <ms>', 'Timeline position in milliseconds', parseInt)
    .option('--bounded-context <name>', 'Bounded context association')
    .action(
      async (
        workUnitId: string,
        text: string,
        options: {
          type?: string;
          timestamp?: number;
          boundedContext?: string;
        }
      ) => {
        try {
          const result = await addExternalSystem({
            workUnitId,
            text,
            type: options.type as any,
            timestamp: options.timestamp,
            boundedContext: options.boundedContext,
          });

          if (!result.success) {
            console.error(
              chalk.red('✗ Failed to add external system:'),
              result.error
            );
            process.exit(1);
          }

          console.log(
            chalk.green(
              `✓ External system added to ${workUnitId} (id: ${result.externalSystemId})`
            )
          );
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to add external system:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
