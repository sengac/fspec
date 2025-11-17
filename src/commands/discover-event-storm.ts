/**
 * discover-event-storm Command
 *
 * Emits Event Storm guidance for domain discovery
 */

import { join } from 'path';
import { existsSync } from 'fs';
import { readFileSync } from 'fs';
import type { Command } from 'commander';
import { wrapInSystemReminder } from '../utils/system-reminder';
import { getEventStormSection } from '../utils/slashCommandSections/eventStorm';
import type { WorkUnitsData } from '../types';
import chalk from 'chalk';

export interface DiscoverEventStormOptions {
  workUnitId: string;
  cwd?: string;
}

/**
 * Emit Event Storm guidance for a work unit
 */
export async function discoverEventStormCommand(
  options: DiscoverEventStormOptions
): Promise<void> {
  const cwd = options.cwd || process.cwd();

  // Validate work unit exists
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  if (!existsSync(workUnitsPath)) {
    console.error(
      chalk.red('✗ spec/work-units.json not found. Run fspec init first.')
    );
    process.exit(1);
  }

  const workUnitsData = JSON.parse(
    readFileSync(workUnitsPath, 'utf-8')
  ) as WorkUnitsData;

  if (!workUnitsData.workUnits[options.workUnitId]) {
    console.error(chalk.red(`✗ Work unit ${options.workUnitId} not found`));
    process.exit(1);
  }

  const workUnit = workUnitsData.workUnits[options.workUnitId];

  // Check work unit is in specifying status
  if (workUnit.status !== 'specifying') {
    console.error(
      chalk.red(
        `✗ Work unit ${options.workUnitId} must be in specifying status (currently: ${workUnit.status})`
      )
    );
    console.error(
      chalk.yellow(
        `  Run: fspec update-work-unit-status ${options.workUnitId} specifying`
      )
    );
    process.exit(1);
  }

  // Emit Event Storm guidance as system-reminder
  const guidance = getEventStormSection();
  const reminder = wrapInSystemReminder(
    `EVENT STORM DISCOVERY - ${options.workUnitId}\n\n${guidance}\n\nWork unit: ${options.workUnitId}\n\nUse the commands listed above to capture Event Storm artifacts.\nWhen done, run: fspec generate-example-mapping-from-event-storm ${options.workUnitId}`
  );

  console.log(
    chalk.green(
      `✓ Event Storm discovery session started for ${options.workUnitId}`
    )
  );
  console.log(reminder);
}

/**
 * Register discover-event-storm command
 */
export function registerDiscoverEventStormCommand(program: Command): void {
  program
    .command('discover-event-storm')
    .description('Start Event Storm discovery session for a work unit')
    .argument('<work-unit-id>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      await discoverEventStormCommand({ workUnitId });
    });
}
