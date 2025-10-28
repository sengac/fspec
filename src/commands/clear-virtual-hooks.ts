import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { cleanupVirtualHookScript } from '../hooks/script-generation';
import { fileManager } from '../utils/file-manager';

interface ClearVirtualHooksOptions {
  workUnitId: string;
  cwd?: string;
}

interface ClearVirtualHooksResult {
  success: boolean;
  clearedCount: number;
}

export async function clearVirtualHooks(
  options: ClearVirtualHooksOptions
): Promise<ClearVirtualHooksResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Count hooks before clearing
  const clearedCount = workUnit.virtualHooks?.length || 0;

  // Clean up script files for each virtual hook
  if (workUnit.virtualHooks) {
    for (const hook of workUnit.virtualHooks) {
      try {
        await cleanupVirtualHookScript({
          workUnitId: options.workUnitId,
          hookName: hook.name,
          projectRoot: cwd,
        });
      } catch {
        // Ignore cleanup errors - script might not exist
      }
    }
  }

  // Clear virtual hooks
  workUnit.virtualHooks = [];

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsFile, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    clearedCount,
  };
}

export function registerClearVirtualHooksCommand(program: Command): void {
  program
    .command('clear-virtual-hooks')
    .description('Clear all virtual hooks from a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        const result = await clearVirtualHooks({ workUnitId });
        console.log(
          chalk.green(
            `✓ Cleared ${result.clearedCount} virtual hook(s) from ${workUnitId}`
          )
        );
      } catch (error: unknown) {
        console.error(
          chalk.red('✗ Failed to clear virtual hooks:'),
          error instanceof Error ? error.message : String(error)
        );
        process.exit(1);
      }
    });
}
