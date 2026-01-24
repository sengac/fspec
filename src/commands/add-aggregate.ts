/**
 * Add aggregate to Event Storm section of work unit
 * Coverage: EXMAP-006
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import type { WorkUnitsData, EventStormAggregate } from '../types';
import { fileManager } from '../utils/file-manager';
import { logger } from '../utils/logger';

export interface AddAggregateOptions {
  workUnitId: string;
  text: string;
  responsibilities?: string; // Comma-separated list
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddAggregateResult {
  success: boolean;
  error?: string;
  aggregateId?: number;
}

/**
 * Add aggregate to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and aggregate ID
 */
export async function addAggregate(
  options: AddAggregateOptions
): Promise<AddAggregateResult> {
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

    // Create aggregate item
    const aggregateId = workUnit.eventStorm.nextItemId;
    const aggregate: EventStormAggregate = {
      id: aggregateId,
      type: 'aggregate',
      color: 'yellow',
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.responsibilities) {
      // Parse comma-separated list into array
      aggregate.responsibilities = options.responsibilities
        .split(',')
        .map(r => r.trim())
        .filter(r => r.length > 0);
    }
    if (options.timestamp !== undefined) {
      aggregate.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      aggregate.boundedContext = options.boundedContext;
    }

    // Add aggregate to items array
    workUnit.eventStorm.items.push(aggregate);
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
      aggregateId,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message || 'Unknown error occurred',
    };
  }
}

export function registerAddAggregateCommand(program: Command): void {
  program
    .command('add-aggregate')
    .description('Add aggregate to Event Storm section of work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'Aggregate text/name')
    .option(
      '--responsibilities <list>',
      'Comma-separated list of responsibilities'
    )
    .option('--timestamp <ms>', 'Timeline timestamp in milliseconds', parseInt)
    .option(
      '--bounded-context <context>',
      'Bounded context for domain association'
    )
    .action(async (workUnitId: string, text: string, options: any) => {
      try {
        const result = await addAggregate({
          workUnitId,
          text,
          responsibilities: options.responsibilities,
          timestamp: options.timestamp,
          boundedContext: options.boundedContext,
        });

        if (!result.success) {
          logger.error(result.error || 'Failed to add aggregate');
          process.exit(1);
        }

        logger.success(
          `Added aggregate "${text}" to ${workUnitId} (ID: ${result.aggregateId})`
        );
      } catch (error: any) {
        logger.error(`Error: ${error.message}`);
        process.exit(1);
      }
    });
}
