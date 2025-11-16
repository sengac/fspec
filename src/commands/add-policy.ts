/**
 * Add policy to Event Storm section of work unit
 * Coverage: EXMAP-007
 */

import { existsSync } from 'fs';
import { join } from 'path';
import type { Command } from 'commander';
import chalk from 'chalk';
import type { WorkUnitsData, EventStormPolicy } from '../types';
import { fileManager } from '../utils/file-manager';

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

    // Create policy item
    const policyId = workUnit.eventStorm.nextItemId;
    const policy: EventStormPolicy = {
      id: policyId,
      type: 'policy',
      color: 'purple',
      text: options.text,
      deleted: false,
      createdAt: new Date().toISOString(),
    };

    // Add optional fields
    if (options.when) {
      policy.when = options.when;
    }
    if (options.then) {
      policy.then = options.then;
    }
    if (options.timestamp !== undefined) {
      policy.timestamp = options.timestamp;
    }
    if (options.boundedContext) {
      policy.boundedContext = options.boundedContext;
    }

    // Add policy to items array and increment ID using transaction
    await fileManager.transaction(workUnitsFile, (data: WorkUnitsData) => {
      const wu = data.workUnits[options.workUnitId];
      if (!wu.eventStorm) {
        wu.eventStorm = {
          level: 'process_modeling',
          items: [],
          nextItemId: 0,
        };
      }
      wu.eventStorm.items.push(policy);
      wu.eventStorm.nextItemId += 1;
    });

    return {
      success: true,
      policyId,
    };
  } catch (error: unknown) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    return {
      success: false,
      error: `Failed to add policy: ${errorMessage}`,
    };
  }
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
