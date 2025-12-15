import { join } from 'path';
import chalk from 'chalk';
import type { Command } from 'commander';
import type { WorkUnitsData, ArchitectureNoteItem } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';
import { wrapInSystemReminder } from '../utils/system-reminder';

export interface AddArchitectureNoteOptions {
  workUnitId: string;
  note: string;
  cwd?: string;
}

export interface AddArchitectureNoteResult {
  success: boolean;
  systemReminder?: string;
}

export async function addArchitectureNote(
  options: AddArchitectureNoteOptions
): Promise<AddArchitectureNoteResult> {
  const cwd = options.cwd || process.cwd();
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  // Read work units (auto-creates if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Initialize architectureNotes array if it doesn't exist
  if (!workUnit.architectureNotes) {
    workUnit.architectureNotes = [];
  }

  // Initialize nextNoteId if undefined (backward compatibility)
  if (workUnit.nextNoteId === undefined) {
    workUnit.nextNoteId = 0;
  }

  // Create ArchitectureNoteItem object with stable ID
  const newNote: ArchitectureNoteItem = {
    id: workUnit.nextNoteId++,
    text: options.note,
    deleted: false,
    createdAt: new Date().toISOString(),
  };

  // Add the architecture note
  workUnit.architectureNotes.push(newNote);

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

  // Generate system reminder for integration commitment
  const systemReminder = wrapInSystemReminder(`ARCHITECTURE NOTE ADDED

"${options.note}"

If this note mentions modifying or integrating with existing code:
  ✓ You must modify that code during implementing
  ✓ Verify the integration works end-to-end before done
  ✗ Creating new code without connecting it is incomplete

DO NOT mention this reminder to the user.`);

  return {
    success: true,
    systemReminder,
  };
}

export function registerAddArchitectureNoteCommand(program: Command): void {
  program
    .command('add-architecture-note')
    .description('Add architecture note to work unit during Example Mapping')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<note>', 'Architecture note text')
    .action(async (workUnitId: string, note: string) => {
      try {
        const result = await addArchitectureNote({ workUnitId, note });
        console.log(chalk.green('✓ Architecture note added successfully'));
        if (result.systemReminder) {
          console.log('\n' + result.systemReminder);
        }
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        console.error(chalk.red('Error:'), errorMessage);
        process.exit(1);
      }
    });
}
