import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface RemoveExampleOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RemoveExampleResult {
  success: boolean;
  removedExample: string;
  remainingCount: number;
}

export async function removeExample(
  options: RemoveExampleOptions
): Promise<RemoveExampleResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate work unit is in specifying state
  if (workUnit.status !== 'specifying') {
    throw new Error(
      `Can only remove examples during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate examples array exists
  if (!workUnit.examples || workUnit.examples.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no examples`);
  }

  // Validate index
  if (options.index < 0 || options.index >= workUnit.examples.length) {
    throw new Error(
      `Invalid index ${options.index}. Valid range: 0-${workUnit.examples.length - 1}`
    );
  }

  // Remove example
  const [removedExample] = workUnit.examples.splice(options.index, 1);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    removedExample,
    remainingCount: workUnit.examples.length,
  };
}

export function registerRemoveExampleCommand(program: Command): void {
  program
    .command('remove-example')
    .description('Remove an example from a work unit by index')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Example index (0-based)')
    .action(async (workUnitId: string, index: string) => {
      try {
        const result = await removeExample({
          workUnitId,
          index: parseInt(index, 10),
        });
        console.log(chalk.green(`✓ Removed example: "${result.removedExample}"`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to remove example:'), error.message);
        process.exit(1);
      }
    });
}
