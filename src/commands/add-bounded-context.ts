/**
 * Add bounded context to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormBoundedContext } from '../types';
import { fileManager } from '../utils/file-manager';

export interface AddBoundedContextOptions {
  workUnitId: string;
  text: string;
  description?: string;
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddBoundedContextResult {
  success: boolean;
  error?: string;
  boundedContextId?: number;
}

/**
 * Add bounded context to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and bounded context ID
 */
export async function addBoundedContext(
  options: AddBoundedContextOptions
): Promise<AddBoundedContextResult> {
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

    // Create bounded context item
    const boundedContextId = workUnit.eventStorm.nextItemId;
    const boundedContext: EventStormBoundedContext = {
      id: boundedContextId,
      type: 'bounded_context',
      color: null,
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.description) {
      boundedContext.description = options.description;
    }
    if (options.timestamp !== undefined) {
      boundedContext.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      boundedContext.boundedContext = options.boundedContext;
    }

    // Add bounded context to items array and increment ID using transaction
    await fileManager.transaction(workUnitsFile, (data: WorkUnitsData) => {
      const wu = data.workUnits[options.workUnitId];
      if (!wu.eventStorm) {
        wu.eventStorm = {
          level: 'process_modeling',
          items: [],
          nextItemId: 0,
        };
      }
      wu.eventStorm.items.push(boundedContext);
      wu.eventStorm.nextItemId += 1;
    });

    return {
      success: true,
      boundedContextId,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: `Failed to add bounded context: ${errorMessage}`,
    };
  }
}

export function registerAddBoundedContextCommand(program: Command): void {
  program
    .command('add-bounded-context')
    .description('Add bounded context to Event Storm section')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'Bounded context name')
    .option(
      '--description <scope>',
      'Scope and responsibilities of the bounded context'
    )
    .option('--timestamp <ms>', 'Timeline position in milliseconds', parseInt)
    .option('--bounded-context <name>', 'Parent bounded context association')
    .action(
      async (
        workUnitId: string,
        text: string,
        options: {
          description?: string;
          timestamp?: number;
          boundedContext?: string;
        }
      ) => {
        try {
          const result = await addBoundedContext({
            workUnitId,
            text,
            ...options,
          });

          if (!result.success) {
            console.error(
              chalk.red('✗ Failed to add bounded context:'),
              result.error
            );
            process.exit(1);
          }

          console.log(
            chalk.green(
              `✓ Bounded context added to ${workUnitId} (id: ${result.boundedContextId})`
            )
          );
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to add bounded context:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
