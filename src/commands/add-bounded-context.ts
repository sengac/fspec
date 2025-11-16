/**
 * Add bounded context to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import type { EventStormBoundedContext } from '../types';
import { addEventStormItem } from './event-storm-utils';

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
  // Build bounded context item data
  const itemData: Omit<
    EventStormBoundedContext,
    'id' | 'deleted' | 'createdAt'
  > = {
    type: 'bounded_context',
    color: null,
    text: options.text,
  };

  // Add optional fields
  if (options.description) {
    itemData.description = options.description;
  }
  if (options.timestamp !== undefined) {
    itemData.timestamp = options.timestamp;
  }
  if (options.boundedContext) {
    itemData.boundedContext = options.boundedContext;
  }

  // Use shared utility to add item
  const result = await addEventStormItem<EventStormBoundedContext>({
    workUnitId: options.workUnitId,
    itemData,
    cwd: options.cwd,
  });

  // Map result to bounded context-specific format
  return {
    success: result.success,
    error: result.error,
    boundedContextId: result.itemId,
  };
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
