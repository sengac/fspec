import { readFile, writeFile, unlink } from 'fs/promises';
import { join, basename } from 'path';
import chalk from 'chalk';
import type { Command } from 'commander';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

export interface RemoveAttachmentOptions {
  workUnitId: string;
  fileName: string;
  keepFile?: boolean;
  cwd?: string;
}

export async function removeAttachment(
  options: RemoveAttachmentOptions
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

  // Check if attachments exist
  if (!workUnit.attachments || workUnit.attachments.length === 0) {
    throw new Error(
      `Work unit '${options.workUnitId}' has no attachments to remove`
    );
  }

  // Find attachment by filename
  const attachmentIndex = workUnit.attachments.findIndex((path) =>
    path.endsWith(options.fileName)
  );

  if (attachmentIndex === -1) {
    throw new Error(
      `Attachment '${options.fileName}' not found for work unit '${options.workUnitId}'`
    );
  }

  const attachmentPath = workUnit.attachments[attachmentIndex];
  const fullPath = join(cwd, attachmentPath);

  // Remove from attachments array
  workUnit.attachments.splice(attachmentIndex, 1);

  // Delete the file unless --keep-file is specified
  if (!options.keepFile) {
    try {
      await unlink(fullPath);
      console.log(chalk.green('✓ Attachment removed from work unit and file deleted'));
    } catch (error: unknown) {
      console.log(
        chalk.yellow('⚠ Attachment removed from work unit (file was already missing)')
      );
    }
  } else {
    console.log(chalk.green('✓ Attachment removed from work unit (file kept)'));
  }

  console.log(chalk.dim(`  File: ${attachmentPath}`));

  // Update timestamp
  workUnit.updatedAt = new Date().toISOString();

  // Update metadata
  if (data.meta) {
    data.meta.lastUpdated = new Date().toISOString();
  }

  // Write back to file
  await writeFile(workUnitsPath, JSON.stringify(data, null, 2));
}

export function registerRemoveAttachmentCommand(program: Command): void {
  program
    .command('remove-attachment')
    .description('Remove attachment from work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<fileName>', 'File name to remove (e.g., diagram.png)')
    .option(
      '--keep-file',
      'Keep the file on disk (only remove from work unit tracking)'
    )
    .action(
      async (
        workUnitId: string,
        fileName: string,
        cmdOptions: { keepFile?: boolean }
      ) => {
        try {
          await removeAttachment({
            workUnitId,
            fileName,
            keepFile: cmdOptions.keepFile,
          });
        } catch (error: unknown) {
          const errorMessage =
            error instanceof Error ? error.message : String(error);
          console.error(chalk.red('Error:'), errorMessage);
          process.exit(1);
        }
      }
    );
}
