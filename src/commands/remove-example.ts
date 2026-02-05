import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

import { output } from '../utils/output';
interface RemoveExampleOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

interface RemoveExampleResult {
  success: boolean;
  removedExample: string;
  remainingCount: number;
  message?: string; // For idempotent operations
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

  // Find example by ID (index is now treated as ID for stable indices)
  const example = workUnit.examples.find(e => e.id === options.index);

  if (!example) {
    throw new Error(`Example with ID ${options.index} not found`);
  }

  // If already deleted, return idempotent success
  if (example.deleted) {
    return {
      success: true,
      removedExample: example.text,
      remainingCount: workUnit.examples.filter(e => !e.deleted).length,
      message: `Item ID ${options.index} already deleted`,
    };
  }

  // Soft-delete: set deleted flag and timestamp
  example.deleted = true;
  example.deletedAt = new Date().toISOString();

  const removedExample = example.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    removedExample,
    remainingCount: workUnit.examples.filter(e => !e.deleted).length,
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
        output.log(
          chalk.green(`✓ Removed example: "${result.removedExample}"`)
        );
      } catch (error: any) {
        output.error('✗ Failed to remove example:', error.message);
        process.exit(1);
      }
    });
}
