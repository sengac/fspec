/**
 * Add policy to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import type { Command } from 'commander';
import chalk from 'chalk';
import type { EventStormPolicy } from '../types';
import { addEventStormItem } from './event-storm-utils';

export interface AddPolicyOptions {
  workUnitId: string;
  text: string;
  when?: string;
  then?: string;
  timestamp?: number;
  boundedContext?: string;
  cwd?: string;
}

export interface AddPolicyResult {
  success: boolean;
  error?: string;
  policyId?: number;
}

/**
 * Add policy to work unit's Event Storm section
 *
 * @param options - Command options
 * @returns Result with success status and policy ID
 */
export async function addPolicy(
  options: AddPolicyOptions
): Promise<AddPolicyResult> {
  // Build policy item data
  const itemData: Omit<EventStormPolicy, 'id' | 'deleted' | 'createdAt'> = {
    type: 'policy',
    color: 'purple',
    text: options.text,
  };

  // Add optional fields
  if (options.when) {
    itemData.when = options.when;
  }
  if (options.then) {
    itemData.then = options.then;
  }
  if (options.timestamp !== undefined) {
    itemData.timestamp = options.timestamp;
  }
  if (options.boundedContext) {
    itemData.boundedContext = options.boundedContext;
  }

  // Use shared utility to add item
  const result = await addEventStormItem<EventStormPolicy>({
    workUnitId: options.workUnitId,
    itemData,
    cwd: options.cwd,
  });

  // Map result to policy-specific format
  return {
    success: result.success,
    error: result.error,
    policyId: result.itemId,
  };
}

export function registerAddPolicyCommand(program: Command): void {
  program
    .command('add-policy')
    .description('Add policy to Event Storm section')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<text>', 'Policy text')
    .option('--when <event>', 'Event that triggers the policy')
    .option('--then <command>', 'Command that executes when policy triggers')
    .option('--timestamp <ms>', 'Timeline position in milliseconds', parseInt)
    .option('--bounded-context <name>', 'Bounded context association')
    .action(
      async (
        workUnitId: string,
        text: string,
        options: {
          when?: string;
          then?: string;
          timestamp?: number;
          boundedContext?: string;
        }
      ) => {
        try {
          const result = await addPolicy({
            workUnitId,
            text,
            ...options,
          });

          if (!result.success) {
            console.error(chalk.red('✗ Failed to add policy:'), result.error);
            process.exit(1);
          }

          console.log(
            chalk.green(
              `✓ Policy added to ${workUnitId} (id: ${result.policyId})`
            )
          );
        } catch (error: any) {
          console.error(chalk.red('✗ Failed to add policy:'), error.message);
          process.exit(1);
        }
      }
    );
}
