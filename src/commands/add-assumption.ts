import { writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface AddAssumptionOptions {
  workUnitId: string;
  assumption: string;
  cwd?: string;
}

interface AddAssumptionResult {
  success: boolean;
  assumptionCount: number;
}

export async function addAssumption(
  options: AddAssumptionOptions
): Promise<AddAssumptionResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate work unit is in specifying state
  if (workUnit.status !== 'specifying') {
    throw new Error(
      `Can only add assumptions during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize assumptions array if it doesn't exist
  if (!workUnit.assumptions) {
    workUnit.assumptions = [];
  }

  // Add assumption
  workUnit.assumptions.push(options.assumption);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    assumptionCount: workUnit.assumptions.length,
  };
}

export function registerAddAssumptionCommand(program: Command): void {
  program
    .command('add-assumption')
    .description('Add assumption to work unit during specification')
    .argument('<work-unit-id>', 'Work unit ID')
    .argument('<assumption>', 'Assumption text')
    .action(async (workUnitId: string, assumption: string) => {
      try {
        await addAssumption({ workUnitId, assumption });
        console.log(chalk.green(`✓ Assumption added successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add assumption:'), error.message);
        process.exit(1);
      }
    });
}
