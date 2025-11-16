/**
 * Add hotspot to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormHotspot } from '../types';
import { fileManager } from '../utils/file-manager';

export interface AddHotspotOptions {
  workUnitId: string;
  text: string;
  concern?: string;
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddHotspotResult {
  success: boolean;
  error?: string;
  hotspotId?: number;
}

/**
 * Add hotspot to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and hotspot ID
 */
export async function addHotspot(
  options: AddHotspotOptions
): Promise<AddHotspotResult> {
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

    // Create hotspot item
    const hotspotId = workUnit.eventStorm.nextItemId;
    const hotspot: EventStormHotspot = {
      id: hotspotId,
      type: 'hotspot',
      color: 'red',
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.concern) {
      hotspot.concern = options.concern;
    }
    if (options.timestamp !== undefined) {
      hotspot.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      hotspot.boundedContext = options.boundedContext;
    }

    // Add hotspot to items array and increment ID using transaction
    await fileManager.transaction(workUnitsFile, (data: WorkUnitsData) => {
      const wu = data.workUnits[options.workUnitId];
      if (!wu.eventStorm) {
        wu.eventStorm = {
          level: 'process_modeling',
          items: [],
          nextItemId: 0,
        };
      }
      wu.eventStorm.items.push(hotspot);
      wu.eventStorm.nextItemId += 1;
    });

    return {
      success: true,
      hotspotId,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: `Failed to add hotspot: ${errorMessage}`,
    };
  }
}

export function registerAddHotspotCommand(program: Command): void {
  program
    .command('add-hotspot')
    .description('Add hotspot to Event Storm section')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'Hotspot description')
    .option(
      '--concern <description>',
      'Risk, uncertainty, or problem description'
    )
    .option('--timestamp <ms>', 'Timeline position in milliseconds', parseInt)
    .option('--bounded-context <name>', 'Bounded context association')
    .action(
      async (
        workUnitId: string,
        text: string,
        options: {
          concern?: string;
          timestamp?: number;
          boundedContext?: string;
        }
      ) => {
        try {
          const result = await addHotspot({
            workUnitId,
            text,
            ...options,
          });

          if (!result.success) {
            console.error(chalk.red('✗ Failed to add hotspot:'), result.error);
            process.exit(1);
          }

          console.log(
            chalk.green(
              `✓ Hotspot added to ${workUnitId} (id: ${result.hotspotId})`
            )
          );
        } catch (error: any) {
          console.error(chalk.red('✗ Failed to add hotspot:'), error.message);
          process.exit(1);
        }
      }
    );
}
