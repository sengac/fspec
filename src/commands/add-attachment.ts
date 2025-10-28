import { mkdir, copyFile, access } from 'fs/promises';
import { join, basename, relative } from 'path';
import chalk from 'chalk';
import type { Command } from 'commander';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';
import {
  shouldValidateMermaid,
  validateMermaidAttachment,
} from '../utils/attachment-mermaid-validation';

export interface AddAttachmentOptions {
  workUnitId: string;
  filePath: string;
  description?: string;
  cwd?: string;
}

export async function addAttachment(
  options: AddAttachmentOptions
): Promise<void> {
  const cwd = options.cwd || process.cwd();
  const workUnitsPath = join(cwd, 'spec', 'work-units.json');

  // Read work units (auto-creates if missing)
  const data: WorkUnitsData = await ensureWorkUnitsFile(cwd);

  // Validate work unit exists
  if (!data.workUnits[options.workUnitId]) {
    throw new Error(`Work unit '${options.workUnitId}' does not exist`);
  }

  // Validate source file exists
  try {
    await access(options.filePath);
  } catch {
    throw new Error(`Source file '${options.filePath}' does not exist`);
  }

  // Validate Mermaid syntax if file is a Mermaid diagram
  if (shouldValidateMermaid(options.filePath)) {
    const fileName = basename(options.filePath);
    try {
      const validationResult = await validateMermaidAttachment(
        options.filePath
      );
      if (!validationResult.valid) {
        throw new Error(
          `Failed to attach ${fileName}: ${validationResult.error}`
        );
      }
    } catch (error: unknown) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      throw new Error(errorMessage);
    }
  }

  const workUnit = data.workUnits[options.workUnitId];

  // Create attachments directory for this work unit
  const attachmentsDir = join(cwd, 'spec', 'attachments', options.workUnitId);
  await mkdir(attachmentsDir, { recursive: true });

  // Copy file to attachments directory
  const fileName = basename(options.filePath);
  const destPath = join(attachmentsDir, fileName);
  await copyFile(options.filePath, destPath);

  // Get relative path from project root
  const relativePath = relative(cwd, destPath);

  // Initialize attachments array if it doesn't exist
  if (!workUnit.attachments) {
    workUnit.attachments = [];
  }

  // Check if attachment already exists
  if (workUnit.attachments.includes(relativePath)) {
    throw new Error(
      `Attachment '${fileName}' already exists for work unit '${options.workUnitId}'`
    );
  }

  // Add the attachment path
  workUnit.attachments.push(relativePath);

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

  console.log(chalk.green('✓ Attachment added successfully'));
  console.log(chalk.dim(`  File: ${relativePath}`));
  if (options.description) {
    console.log(chalk.dim(`  Description: ${options.description}`));
  }
}

export function registerAddAttachmentCommand(program: Command): void {
  program
    .command('add-attachment')
    .description(
      'Add attachment to work unit during Example Mapping (diagrams, mockups, documents, etc.)'
    )
    .argument('<workUnitId>', 'Work unit ID')
    .argument('<filePath>', 'Path to file to attach')
    .option(
      '-d, --description <text>',
      'Optional description of the attachment'
    )
    .action(
      async (
        workUnitId: string,
        filePath: string,
        cmdOptions: { description?: string }
      ) => {
        try {
          await addAttachment({
            workUnitId,
            filePath,
            description: cmdOptions.description,
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
