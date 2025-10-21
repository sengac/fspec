import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, VirtualHook } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

interface ListVirtualHooksOptions {
  workUnitId: string;
  cwd?: string;
}

interface ListVirtualHooksResult {
  hooks: VirtualHook[];
  hooksByEvent: Record<string, VirtualHook[]>;
}

export async function listVirtualHooks(
  options: ListVirtualHooksOptions
): Promise<ListVirtualHooksResult> {
  const cwd = options.cwd || process.cwd();

  // Read work units
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];
  const hooks = workUnit.virtualHooks || [];

  // Group hooks by event
  const hooksByEvent: Record<string, VirtualHook[]> = {};
  for (const hook of hooks) {
    if (!hooksByEvent[hook.event]) {
      hooksByEvent[hook.event] = [];
    }
    hooksByEvent[hook.event].push(hook);
  }

  return {
    hooks,
    hooksByEvent,
  };
}

export function registerListVirtualHooksCommand(program: Command): void {
  program
    .command('list-virtual-hooks')
    .description('List all virtual hooks for a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        const result = await listVirtualHooks({ workUnitId });

        if (result.hooks.length === 0) {
          console.log(
            chalk.yellow(`No virtual hooks configured for ${workUnitId}`)
          );
          return;
        }

        console.log(chalk.bold(`\nVirtual Hooks for ${workUnitId}:\n`));

        // Display hooks grouped by event
        for (const [event, hooks] of Object.entries(result.hooksByEvent)) {
          console.log(chalk.cyan(`  ${event}:`));
          for (const hook of hooks) {
            const blockingBadge = hook.blocking
              ? chalk.red('[blocking]')
              : chalk.gray('[non-blocking]');
            const gitContextBadge = hook.gitContext
              ? chalk.blue('[git-context]')
              : '';
            console.log(
              `    • ${hook.name} ${blockingBadge} ${gitContextBadge}`
            );
            console.log(chalk.gray(`      ${hook.command}`));
          }
        }
        console.log();
      } catch (error: unknown) {
        console.error(
          chalk.red('✗ Failed to list virtual hooks:'),
          error instanceof Error ? error.message : String(error)
        );
        process.exit(1);
      }
    });
}
