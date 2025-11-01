import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface RestoreExampleOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RestoreExampleResult {
  success: boolean;
  restoredExample: string;
  activeCount: number;
  message?: string; // For idempotent operations
}

export async function restoreExample(
  options: RestoreExampleOptions
): Promise<RestoreExampleResult> {
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
      `Can only restore examples during discovery/specification phase. ${options.workUnitId} is in '${workUnit.status}' state.`
    );
  }

  // Validate examples array exists
  if (!workUnit.examples || workUnit.examples.length === 0) {
    throw new Error(`Work unit ${options.workUnitId} has no examples`);
  }

  // Find example by ID (index is now treated as ID for stable indices)
  const example = workUnit.examples.find(e => e.id === options.index);

  if (!example) {
    throw new Error(`Example with ID ${options.index} not found`);
  }

  // If already active, return idempotent success
  if (!example.deleted) {
    return {
      success: true,
      restoredExample: example.text,
      activeCount: workUnit.examples.filter(e => !e.deleted).length,
      message: `Item ID ${options.index} already active`,
    };
  }

  // Restore: clear deleted flag and timestamp
  example.deleted = false;
  delete example.deletedAt;

  const restoredExample = example.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    restoredExample,
    activeCount: workUnit.examples.filter(e => !e.deleted).length,
  };
}

export function registerRestoreExampleCommand(program: Command): void {
  program
    .command('restore-example')
    .description('Restore a soft-deleted example by ID')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Example ID (0-based)')
    .action(async (workUnitId: string, index: string) => {
      try {
        const result = await restoreExample({
          workUnitId,
          index: parseInt(index, 10),
        });
        console.log(
          chalk.green(`✓ Restored example: "${result.restoredExample}"`)
        );
        if (result.message) {
          console.log(chalk.dim(`  ${result.message}`));
        }
      } catch (error: any) {
        console.error(chalk.red('✗ Failed to restore example:'), error.message);
        process.exit(1);
      }
    });
}
