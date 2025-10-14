import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  actualTokens?: number;
  updatedAt?: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function recordMetric(options: {
  workUnitId: string;
  tokens: number;
  cwd?: string;
}): Promise<{ success: boolean; totalTokens?: number }> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    // Read work units
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    // Check if work unit exists
    if (!data.workUnits[options.workUnitId]) {
      throw new Error(`Work unit ${options.workUnitId} not found`);
    }

    // Add tokens to actualTokens (cumulative)
    const workUnit = data.workUnits[options.workUnitId];
    workUnit.actualTokens = (workUnit.actualTokens || 0) + options.tokens;
    workUnit.updatedAt = new Date().toISOString();

    // Write back to file
    await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

    return { success: true, totalTokens: workUnit.actualTokens };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to record metric: ${error.message}`);
    }
    throw error;
  }
}

export function registerRecordMetricCommand(program: Command): void {
  program
    .command('record-metric')
    .description('Record a project metric')
    .argument('<metric>', 'Metric name')
    .argument('<value>', 'Metric value')
    .option('--unit <unit>', 'Unit of measurement')
    .action(async (metric: string, value: string, options: { unit?: string }) => {
      try {
        await recordMetric({
          metric,
          value: parseFloat(value),
          unit: options.unit,
        });
        console.log(chalk.green(`✓ Metric recorded successfully`));
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to record metric:'), error.message);
        process.exit(1);
      }
    });
}
