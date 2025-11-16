/**
 * Add external system to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import type { EventStormExternalSystem } from '../types';
import { addEventStormItem } from './event-storm-utils';

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
  // Build external system item data
  const itemData: Omit<
    EventStormExternalSystem,
    'id' | 'deleted' | 'createdAt'
  > = {
    type: 'external_system',
    color: 'pink',
    text: options.text,
  };

  // Add optional fields
  if (options.type) {
    itemData.integrationType = options.type;
  }
  if (options.timestamp !== undefined) {
    itemData.timestamp = options.timestamp;
  }
  if (options.boundedContext) {
    itemData.boundedContext = options.boundedContext;
  }

  // Use shared utility to add item
  const result = await addEventStormItem<EventStormExternalSystem>({
    workUnitId: options.workUnitId,
    itemData,
    cwd: options.cwd,
  });

  // Map result to external system-specific format
  return {
    success: result.success,
    error: result.error,
    externalSystemId: result.itemId,
  };
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
