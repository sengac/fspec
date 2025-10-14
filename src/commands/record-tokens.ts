import { readFile, writeFile } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';

interface WorkUnit {
  id: string;
  actualTokens?: number;
  updatedAt: string;
  [key: string]: unknown;
}

interface WorkUnitsData {
  workUnits: Record<string, WorkUnit>;
  states: Record<string, string[]>;
}

export async function recordTokens(options: {
  workUnitId: string;
  tokens: number;
  cwd?: string;
}): Promise<{ success: boolean; totalTokens?: number }> {
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

    // Add tokens
    const workUnit = data.workUnits[options.workUnitId];
    workUnit.actualTokens = (workUnit.actualTokens || 0) + options.tokens;
    workUnit.updatedAt = new Date().toISOString();

    // Save work units
    await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

    return {
      success: true,
      totalTokens: workUnit.actualTokens,
    };
  } catch (error: unknown) {
    if (error instanceof Error) {
      throw new Error(`Failed to record tokens: ${error.message}`);
    }
    throw error;
  }
}

export function registerRecordTokensCommand(program: Command): void {
  program
    .command('record-tokens')
    .description('Record token usage for AI operations')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<tokens>', 'Number of tokens used')
    .option(
      '--operation <operation>',
      'Operation type (e.g., specification, implementation)'
    )
    .action(
      async (
        workUnitId: string,
        tokens: string,
        options: { operation?: string }
      ) => {
        try {
          await recordTokens({
            workUnitId,
            tokens: parseInt(tokens, 10),
            operation: options.operation,
          });
          console.log(chalk.green(`✓ Token usage recorded successfully`));
        } catch (error: any) {
          console.error(chalk.red('✗ Failed to record tokens:'), error.message);
          process.exit(1);
        }
      }
    );
}
