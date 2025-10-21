import { writeFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData, VirtualHook } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { generateVirtualHookScript } from '../hooks/script-generation';

interface AddVirtualHookOptions {
  workUnitId: string;
  event: string;
  command: string;
  blocking?: boolean;
  gitContext?: boolean;
  cwd?: string;
}

interface AddVirtualHookResult {
  success: boolean;
  hookCount: number;
}

export async function addVirtualHook(
  options: AddVirtualHookOptions
): Promise<AddVirtualHookResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsFile = join(cwd, 'spec/work-units.json');

  // Read work units (auto-creates file if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Initialize virtualHooks array if it doesn't exist
  if (!workUnit.virtualHooks) {
    workUnit.virtualHooks = [];
  }

  // Generate hook name from command (e.g., "eslint src/" -> "eslint")
  const hookName = options.command.split(' ')[0].split('/').pop() || 'hook';

  // Determine the actual command to store
  let commandToStore = options.command;

  // If git context is requested, generate a script file
  if (options.gitContext) {
    const scriptPath = await generateVirtualHookScript({
      workUnitId: options.workUnitId,
      hookName,
      command: options.command,
      gitContext: true,
      projectRoot: cwd,
    });

    // Store script path as command (relative to project root)
    commandToStore = scriptPath.replace(cwd + '/', '');
  }

  // Create virtual hook
  const virtualHook: VirtualHook = {
    name: hookName,
    event: options.event,
    command: commandToStore,
    blocking: options.blocking ?? false,
  };

  // Add optional gitContext flag
  if (options.gitContext) {
    virtualHook.gitContext = true;
  }

  // Add virtual hook
  workUnit.virtualHooks.push(virtualHook);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Write updated work units
  await writeFile(workUnitsFile, JSON.stringify(data, null, 2));

  return {
    success: true,
    hookCount: workUnit.virtualHooks.length,
  };
}

export function registerAddVirtualHookCommand(program: Command): void {
  program
    .command('add-virtual-hook')
    .description('Add a virtual hook to a work unit for dynamic validation')
    .argument('<workUnitId>', 'Work unit ID')
    .argument(
      '<event>',
      'Hook event (e.g., post-implementing, post-validating)'
    )
    .argument('<command>', 'Command to execute')
    .option('--blocking', 'Block workflow transition if hook fails', false)
    .option(
      '--git-context',
      'Provide git context (staged/unstaged files)',
      false
    )
    .action(
      async (
        workUnitId: string,
        event: string,
        command: string,
        opts: { blocking?: boolean; gitContext?: boolean }
      ) => {
        try {
          const result = await addVirtualHook({
            workUnitId,
            event,
            command,
            blocking: opts.blocking,
            gitContext: opts.gitContext,
          });
          console.log(chalk.green(`✓ Virtual hook added to ${workUnitId}`));
          console.log(chalk.gray(`  Total virtual hooks: ${result.hookCount}`));
        } catch (error: unknown) {
          console.error(
            chalk.red('✗ Failed to add virtual hook:'),
            error instanceof Error ? error.message : String(error)
          );
          process.exit(1);
        }
      }
    );
}
