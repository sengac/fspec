import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, VirtualHook } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface CopyVirtualHooksOptions {
  from: string;
  to: string;
  hookName?: string;
  cwd?: string;
}

interface CopyVirtualHooksResult {
  success: boolean;
  copiedCount: number;
}

export async function copyVirtualHooks(
  options: CopyVirtualHooksOptions
): Promise<CopyVirtualHooksResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate source work unit exists
  if (!data.workUnits[options.from]) {
    throw new Error(`Source work unit '${options.from}' does not exist`);
  }

  // Validate target work unit exists
  if (!data.workUnits[options.to]) {
    throw new Error(`Target work unit '${options.to}' does not exist`);
  }

  const sourceWorkUnit = data.workUnits[options.from];
  const targetWorkUnit = data.workUnits[options.to];

  // Check if source has virtual hooks
  if (
    !sourceWorkUnit.virtualHooks ||
    sourceWorkUnit.virtualHooks.length === 0
  ) {
    throw new Error(
      `No virtual hooks configured for source work unit ${options.from}`
    );
  }

  // Initialize target virtualHooks array if it doesn't exist
  if (!targetWorkUnit.virtualHooks) {
    targetWorkUnit.virtualHooks = [];
  }

  let hooksToCopy: VirtualHook[];

  // If hookName specified, copy only that hook
  if (options.hookName) {
    const hook = sourceWorkUnit.virtualHooks.find(
      h => h.name === options.hookName
    );
    if (!hook) {
      throw new Error(
        `Hook '${options.hookName}' not found in ${options.from}`
      );
    }
    hooksToCopy = [hook];
  } else {
    // Copy all hooks
    hooksToCopy = sourceWorkUnit.virtualHooks;
  }

  // Deep copy hooks to avoid reference issues
  const copiedHooks = hooksToCopy.map(hook => ({ ...hook }));

  // Add copied hooks to target
  targetWorkUnit.virtualHooks.push(...copiedHooks);

  // Update timestamp
  targetWorkUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    copiedCount: copiedHooks.length,
  };
}

export function registerCopyVirtualHooksCommand(program: Command): void {
  program
    .command('copy-virtual-hooks')
    .description('Copy virtual hooks from one work unit to another')
    .option('--from <workUnitId>', 'Source work unit ID')
    .option('--to <workUnitId>', 'Target work unit ID')
    .option('--hook-name <name>', 'Copy only specific hook (optional)')
    .action(async (opts: { from?: string; to?: string; hookName?: string }) => {
      try {
        if (!opts.from) {
          throw new Error('--from option is required');
        }
        if (!opts.to) {
          throw new Error('--to option is required');
        }

        const result = await copyVirtualHooks({
          from: opts.from,
          to: opts.to,
          hookName: opts.hookName,
        });

        console.log(
          chalk.green(
            `✓ Copied ${result.copiedCount} virtual hook(s) from ${opts.from} to ${opts.to}`
          )
        );
      } catch (error: unknown) {
        console.error(
          chalk.red('✗ Failed to copy virtual hooks:'),
          error instanceof Error ? error.message : String(error)
        );
        process.exit(1);
      }
    });
}
