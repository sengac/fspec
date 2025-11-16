/**
 * Add hotspot to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import type { EventStormHotspot } from '../types';
import { addEventStormItem } from './event-storm-utils';

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
  // Build hotspot item data
  const itemData: Omit<EventStormHotspot, 'id' | 'deleted' | 'createdAt'> = {
    type: 'hotspot',
    color: 'red',
    text: options.text,
  };

  // Add optional fields
  if (options.concern) {
    itemData.concern = options.concern;
  }
  if (options.timestamp !== undefined) {
    itemData.timestamp = options.timestamp;
  }
  if (options.boundedContext) {
    itemData.boundedContext = options.boundedContext;
  }

  // Use shared utility to add item
  const result = await addEventStormItem<EventStormHotspot>({
    workUnitId: options.workUnitId,
    itemData,
    cwd: options.cwd,
  });

  // Map result to hotspot-specific format
  return {
    success: result.success,
    error: result.error,
    hotspotId: result.itemId,
  };
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
