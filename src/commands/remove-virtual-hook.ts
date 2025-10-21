import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { cleanupVirtualHookScript } from '../hooks/script-generation';

interface RemoveVirtualHookOptions {
  workUnitId: string;
  hookName: string;
  cwd?: string;
}

interface RemoveVirtualHookResult {
  success: boolean;
  remainingCount: number;
}

export async function removeVirtualHook(
  options: RemoveVirtualHookOptions
): Promise<RemoveVirtualHookResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Check if virtualHooks array exists
  if (!workUnit.virtualHooks || workUnit.virtualHooks.length === 0) {
    throw new Error(`No virtual hooks configured for ${options.workUnitId}`);
  }

  // Find and remove the hook
  const initialLength = workUnit.virtualHooks.length;
  workUnit.virtualHooks = workUnit.virtualHooks.filter(
    hook => hook.name !== options.hookName
  );

  // Check if hook was found and removed
  if (workUnit.virtualHooks.length === initialLength) {
    throw new Error(
      `Virtual hook '${options.hookName}' not found in ${options.workUnitId}`
    );
  }

  // Clean up script file if it exists (ignore errors)
  try {
    await cleanupVirtualHookScript({
      workUnitId: options.workUnitId,
      hookName: options.hookName,
      projectRoot: cwd,
    });
  } catch {
    // Ignore cleanup errors - script might not exist
  }

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    remainingCount: workUnit.virtualHooks.length,
  };
}

export function registerRemoveVirtualHookCommand(program: Command): void {
  program
    .command('remove-virtual-hook')
    .description('Remove a virtual hook from a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<hookName>', 'Name of the hook to remove')
    .action(async (workUnitId: string, hookName: string) => {
      try {
        const result = await removeVirtualHook({ workUnitId, hookName });
        console.log(
          chalk.green(`✓ Removed virtual hook '${hookName}' from ${workUnitId}`)
        );
        console.log(
          chalk.gray(`  Remaining virtual hooks: ${result.remainingCount}`)
        );
      } catch (error: unknown) {
        console.error(
          chalk.red('✗ Failed to remove virtual hook:'),
          error instanceof Error ? error.message : String(error)
        );
        process.exit(1);
      }
    });
}
