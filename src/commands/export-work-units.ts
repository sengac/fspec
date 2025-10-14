import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  title?: string;
  status?: string;
  epic?: string;
  estimate?: number;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function exportWorkUnits(options: {
  format: string;
  output: string;
  cwd?: string;
}): Promise<{ success: boolean }> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec', 'work-units.json');

  try {
    const content = await readFile(workUnitsFile, 'utf-8');
    const data: WorkUnitsData = JSON.parse(content);

    const workUnits = Object.values(data.workUnits);

    if (options.format === 'json') {
      await writeFile(options.output, JSON.stringify(workUnits, null, 2));
    } else {
      throw new Error(`Unsupported format: ${options.format}`);
    }

    return { success: true };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to export work units: ${error.message}`);
    }
    throw error;
  }
}

export function registerExportWorkUnitsCommand(program: Command): void {
  program
    .command('export-work-units')
    .description('Export work units to JSON or CSV')
    .argument('<format>', 'Output format: json or csv')
    .argument('<output>', 'Output file path')
    .option('--status <status>', 'Filter by status')
    .action(
      async (format: string, output: string, options: { status?: string }) => {
        try {
          const result = await exportWorkUnits({
            format: format as 'json' | 'csv',
            output,
            status: options.status as any,
          });
          console.log(
            chalk.green(
              `✓ Exported ${result.count} work units to ${result.outputFile}`
            )
          );
        } catch (error: any) {
          console.error(
            chalk.red('✗ Failed to export work units:'),
            error.message
          );
          process.exit(1);
        }
      }
    );
}
