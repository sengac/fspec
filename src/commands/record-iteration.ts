import { readFile, writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  iterations?: number;
  updatedAt: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function recordIteration(options: {
  workUnitId: string;
  cwd?: string;
}): Promise<{ success: boolean; iterations?: number }> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Load work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Check if work unit exists
    if (!data.workUnits[options.workUnitId]) {
      throw new Error(`Work unit ${options.workUnitId} not found`);
    }

    // Increment iterations
    const workUnit = data.workUnits[options.workUnitId];
    workUnit.iterations = (workUnit.iterations || 0) + 1;
    workUnit.updatedAt = new Date().toISOString();

    // Save work units
    await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

    return {
      success: true,
      iterations: workUnit.iterations,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to record iteration: ${error.message}`);
    }
    throw error;
  }
}

export function registerRecordIterationCommand(program: Command): void {
  program
    .command('record-iteration')
    .description('Record an iteration or sprint')
    .argument('<name>', 'Iteration name')
    .option('--start <date>', 'Start date')
    .option('--end <date>', 'End date')
    .action(async (name: string, options: { start?: string; end?: string }) => {
      try {
        await recordIteration({
          name,
          start: options.start,
          end: options.end,
        });
        console.log(chalk.green(`✓ Iteration recorded successfully`));
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to record iteration:'),
          error.message
        );
        process.exit(1);
      }
    });
}
