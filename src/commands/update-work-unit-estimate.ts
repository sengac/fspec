import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  estimate?: number;
  updatedAt?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

const FIBONACCI_NUMBERS = [1, 2, 3, 5, 8, 13, 21];

export async function updateWorkUnitEstimate(options: {
  workUnitId: string;
  estimate: number;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Validate Fibonacci number
    if (!FIBONACCI_NUMBERS.includes(options.estimate)) {
      throw new Error(
        `Invalid estimate: ${options.estimate}. Must be one of: ${FIBONACCI_NUMBERS.join(',')}`
      );
    }

    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Check if work unit exists
    if (!data.workUnits[options.workUnitId]) {
      throw new Error(`Work unit ${options.workUnitId} not found`);
    }

    // Update estimate
    data.workUnits[options.workUnitId].estimate = options.estimate;
    data.workUnits[options.workUnitId].updatedAt = new Date().toISOString();

    // Write back to file
    await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to update work unit estimate: ${error.message}`);
    }
    throw error;
  }
}

export function registerUpdateWorkUnitEstimateCommand(program: Command): void {
  program
    .command('update-work-unit-estimate')
    .description('Update work unit estimate (Fibonacci: 1,2,3,5,8,13,21)')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<estimate>', 'Story points estimate (Fibonacci number)')
    .action(async (workUnitId: string, estimate: string) => {
      try {
        await updateWorkUnitEstimate({
          workUnitId,
          estimate: parseInt(estimate, 10),
        });
        console.log(
          chalk.green(`✓ Work unit ${workUnitId} estimate set to ${estimate}`)
        );
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to update estimate:'), error.message);
        process.exit(1);
      }
    });
}
