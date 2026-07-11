import { mkdir, copyFile, access, unlink, realpath } from 'fs/promises';
import { join, basename, relative, dirname, resolve } from 'path';
import type { Command } from 'commander';
import type { WorkUnitsData } from '../types';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import { fileManager } from '../utils/file-manager';
import { output } from '../utils/output';
import {
  shouldValidateMermaid,
  validateMermaidAttachment,
} from '../utils/attachment-mermaid-validation';
import { resolveFilePath } from '../utils/normalize-path';

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

  // BUG-130: Use resolveFilePath to handle Unicode whitespace variants
  // (e.g. user types regular space but file has U+202F from macOS screenshot)
  const resolvedPath = await resolveFilePath(options.filePath);
  try {
    await access(resolvedPath);
  } catch {
    throw new Error(`Source file '${options.filePath}' does not exist`);
  }

  // Validate Mermaid syntax if file is a Mermaid diagram
  if (shouldValidateMermaid(resolvedPath)) {
    const fileName = basename(resolvedPath);
    try {
      const validationResult = await validateMermaidAttachment(resolvedPath);
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

  // Attachments directory and destination path for this work unit
  const attachmentsDir = join(cwd, 'spec', 'attachments', options.workUnitId);
  const fileName = basename(resolvedPath);
  const destPath = join(attachmentsDir, fileName);

  // Get relative path from project root
  const relativePath = relative(cwd, destPath);

  // BUG-151: Duplicate-registration check BEFORE any filesystem mutation,
  // so a duplicate add-attachment throws without touching the file
  if (workUnit.attachments?.includes(relativePath)) {
    throw new Error(
      `Attachment '${fileName}' already exists for work unit '${options.workUnitId}'`
    );
  }

  await mkdir(attachmentsDir, { recursive: true });

  // BUG-151: Guard source === destination. Canonicalize both paths via
  // realpath (defeats symlink aliasing); fall back to resolve() if
  // canonicalization fails. When equal, register-only: no copy, no unlink —
  // copying a file onto itself truncates it to 0 bytes on some platforms.
  let canonicalSource: string;
  let canonicalDestDir: string;
  try {
    canonicalSource = await realpath(resolvedPath);
    canonicalDestDir = await realpath(attachmentsDir);
  } catch {
    canonicalSource = resolve(resolvedPath);
    canonicalDestDir = resolve(attachmentsDir);
  }
  const isSelfCopy = canonicalSource === join(canonicalDestDir, fileName);

  if (!isSelfCopy) {
    // Copy file to attachments directory (use resolved path for the actual file)
    await copyFile(resolvedPath, destPath);

    // BUG-055: Check if source file is in spec/attachments/ root directory
    // If so, delete it after successful copy to prevent duplication.
    // Never runs on the register-only (self-copy) path.
    const sourceAbsPath = resolve(resolvedPath);
    const attachmentsRootDir = resolve(cwd, 'spec', 'attachments');
    const sourceDir = dirname(sourceAbsPath);

    if (sourceDir === attachmentsRootDir) {
      // Source is in spec/attachments/ root - delete it to prevent duplication
      await unlink(resolvedPath);
    }
  }

  // Initialize attachments array if it doesn't exist
  if (!workUnit.attachments) {
    workUnit.attachments = [];
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

  output.log('✓ Attachment added successfully');
  output.log(`  File: ${relativePath}`);
  if (options.description) {
    output.log(`  Description: ${options.description}`);
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
          output.error('Error:', errorMessage);
          process.exit(1);
        }
      }
    );
}
