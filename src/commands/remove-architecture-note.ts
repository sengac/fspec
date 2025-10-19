import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import type { Command } from 'commander';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

export interface RemoveArchitectureNoteOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

export async function removeArchitectureNote(
  options: RemoveArchitectureNoteOptions
): Promise<void> {
  const cwd = options.cwd || process.cwd();
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  // Read work units (auto-creates if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Validate architectureNotes exists and has items
  if (!workUnit.architectureNotes || workUnit.architectureNotes.length === 0) {
    throw new Error(
      `Work unit '${options.workUnitId}' has no architecture notes`
    );
  }

  // Validate index is within bounds
  if (options.index < 0 || options.index >= workUnit.architectureNotes.length) {
    throw new Error(
      `Invalid index ${options.index}. Work unit has ${workUnit.architectureNotes.length} architecture note(s)`
    );
  }

  // Remove the architecture note at the specified index
  workUnit.architectureNotes.splice(options.index, 1);

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Update metadata
  if (data.meta) {
    data.meta.lastUpdated = new Date().toISOString();
  }

  // Write back to file
  await writeFile(workUnitsPath, JSON.stringify(data, null, 2));
}

export function registerRemoveArchitectureNoteCommand(program: Command): void {
  program
    .command('remove-architecture-note')
    .description('Remove architecture note from work unit by index')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'Index of note to remove (0-based)', parseInt)
    .action(async (workUnitId: string, index: number) => {
      try {
        await removeArchitectureNote({ workUnitId, index });
        console.log(chalk.green('✓ Architecture note removed successfully'));
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        console.error(chalk.red('Error:'), errorMessage);
        process.exit(1);
      }
    });
}
