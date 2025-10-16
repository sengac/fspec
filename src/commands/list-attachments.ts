import { readFile, stat } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import type { Command } from 'commander';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';

export interface ListAttachmentsOptions {
  workUnitId: string;
  cwd?: string;
}

export async function listAttachments(
  options: ListAttachmentsOptions
): Promise<void> {
  const cwd = options.cwd || process.cwd();

  // Read work units (auto-creates if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Check if attachments exist
  if (!workUnit.attachments || workUnit.attachments.length === 0) {
    console.log(
      chalk.yellow(`No attachments found for work unit ${options.workUnitId}`)
    );
    return;
  }

  // Display attachments
  console.log(
    chalk.bold(`\nAttachments for ${options.workUnitId} (${workUnit.attachments.length}):\n`)
  );

  for (const attachment of workUnit.attachments) {
    const fullPath = join(cwd, attachment);

    try {
      const stats = await stat(fullPath);
      const sizeKB = (stats.size / 1024).toFixed(2);

      console.log(chalk.green('  ✓'), chalk.cyan(attachment));
      console.log(chalk.dim(`    Size: ${sizeKB} KB`));
      console.log(
        chalk.dim(`    Modified: ${stats.mtime.toLocaleString()}\n`)
      );
    } catch {
      // File doesn't exist on filesystem
      console.log(chalk.red('  ✗'), chalk.red(attachment));
      console.log(chalk.dim('    File not found on filesystem\n'));
    }
  }
}

export function registerListAttachmentsCommand(program: Command): void {
  program
    .command('list-attachments')
    .description('List all attachments for a work unit')
    .argument('<workUnitId>', 'Work unit ID')
    .action(async (workUnitId: string) => {
      try {
        await listAttachments({ workUnitId });
      } catch (error: unknown) {
        const errorMessage =
          error instanceof Error ? error.message : String(error);
        console.error(chalk.red('Error:'), errorMessage);
        process.exit(1);
      }
    });
}
