import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';

export interface RestoreArchitectureNoteOptions {
  workUnitId: string;
  index: number;
  cwd?: string;
}

export interface RestoreArchitectureNoteResult {
  success: boolean;
  restoredNote: string;
  activeCount: number;
  message?: string; // For idempotent operations
}

export async function restoreArchitectureNote(
  options: RestoreArchitectureNoteOptions
): Promise<RestoreArchitectureNoteResult> {
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

  // Find note by ID (index is now treated as ID for stable indices)
  const note = workUnit.architectureNotes.find(n => n.id === options.index);

  if (!note) {
    throw new Error(`Architecture note with ID ${options.index} not found`);
  }

  // If already active, return idempotent success
  if (!note.deleted) {
    return {
      success: true,
      restoredNote: note.text,
      activeCount: workUnit.architectureNotes.filter(n => !n.deleted).length,
      message: `Item ID ${options.index} already active`,
    };
  }

  // Restore: clear deleted flag and timestamp
  note.deleted = false;
  delete note.deletedAt;

  const restoredNote = note.text;

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Update metadata
  if (data.meta) {
    data.meta.lastUpdated = new Date().toISOString();
  }

  // LOCK-002: Use fileManager.transaction() for atomic write
  await fileManager.transaction(workUnitsPath, async fileData => {
    Object.assign(fileData, data);
  });

  return {
    success: true,
    restoredNote,
    activeCount: workUnit.architectureNotes.filter(n => !n.deleted).length,
  };
}

export function registerRestoreArchitectureNoteCommand(program: Command): void {
  program
    .command('restore-architecture-note')
    .description('Restore soft-deleted architecture note by ID')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<index>', 'ID of note to restore (0-based)', parseInt)
    .action(async (workUnitId: string, index: number) => {
      try {
        const result = await restoreArchitectureNote({ workUnitId, index });
        console.log(chalk.green('✓ Architecture note restored successfully'));
        if (result.message) {
          console.log(chalk.dim(`  ${result.message}`));
        }
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        console.error(chalk.red('Error:'), errorMessage);
        process.exit(1);
      }
    });
}
