import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface AddExampleOptions {
  workUnitId: string;
  example: string;
  cwd?: string;
}

interface AddExampleResult {
  success: boolean;
  exampleCount: number;
}

export async function addExample(
  options: AddExampleOptions
): Promise<AddExampleResult> {
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
      `Can only add examples during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Initialize examples array if it doesn't exist
  if (!workUnit.examples) {
    workUnit.examples = [];
  }

  // Add example
  workUnit.examples.push(options.example);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    exampleCount: workUnit.examples.length,
  };
}

export function registerAddExampleCommand(program: Command): void {
  program
    .command('add-example')
    .description('Add an example to a work unit during specification phase')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<example>', 'Example description')
    .action(async (workUnitId: string, example: string) => {
      try {
        await addExample({ workUnitId, example });
        console.log(chalk.green(`✓ Example added successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to add example:'), error.message);
        process.exit(1);
      }
    });
}
