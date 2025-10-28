import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

interface ClearDependenciesOptions {
  workUnitId: string;
  confirm?: boolean;
  cwd?: string;
}

interface ClearDependenciesResult {
  success: boolean;
}

export async function clearDependencies(
  options: ClearDependenciesOptions
): Promise<ClearDependenciesResult> {
  const cwd = options.cwd || process.cwd();

  if (!options.confirm) {
    throw new Error(
      'Must confirm clearing all dependencies with --confirm flag'
    );
  }

  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Remove bidirectional blocks relationships
  if (workUnit.blocks) {
    for (const targetId of workUnit.blocks) {
      if (data.workUnits[targetId]?.blockedBy) {
        data.workUnits[targetId].blockedBy = data.workUnits[
          targetId
        ].blockedBy!.filter(id => id !== options.workUnitId);
        if (data.workUnits[targetId].blockedBy!.length === 0) {
          delete data.workUnits[targetId].blockedBy;
        }
      }
    }
    delete workUnit.blocks;
  }

  // Remove bidirectional blockedBy relationships
  if (workUnit.blockedBy) {
    for (const targetId of workUnit.blockedBy) {
      if (data.workUnits[targetId]?.blocks) {
        data.workUnits[targetId].blocks = data.workUnits[
          targetId
        ].blocks!.filter(id => id !== options.workUnitId);
        if (data.workUnits[targetId].blocks!.length === 0) {
          delete data.workUnits[targetId].blocks;
        }
      }
    }
    delete workUnit.blockedBy;
  }

  // Remove unidirectional dependsOn relationships
  if (workUnit.dependsOn) {
    delete workUnit.dependsOn;
  }

  // Remove bidirectional relatesTo relationships
  if (workUnit.relatesTo) {
    for (const targetId of workUnit.relatesTo) {
      if (data.workUnits[targetId]?.relatesTo) {
        data.workUnits[targetId].relatesTo = data.workUnits[
          targetId
        ].relatesTo!.filter(id => id !== options.workUnitId);
        if (data.workUnits[targetId].relatesTo!.length === 0) {
          delete data.workUnits[targetId].relatesTo;
        }
      }
    }
    delete workUnit.relatesTo;
  }

  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
  };
}

export function registerClearDependenciesCommand(program: Command): void {
  program
    .command('clear-dependencies')
    .description('Remove all dependencies from a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .option('--confirm', 'Confirm clearing all dependencies')
    .action(async (workUnitId: string, options: { confirm?: boolean }) => {
      try {
        await clearDependencies({
          workUnitId,
          confirm: options.confirm,
        });
        console.log(
          chalk.green(`✓ All dependencies cleared from ${workUnitId}`)
        );
      } catch (error: any) {
        console.error(
          chalk.red('✗ Failed to clear dependencies:'),
          error.message
        );
        process.exit(1);
      }
    });
}
