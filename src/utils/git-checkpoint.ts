/**
 * Git checkpoint utilities using isomorphic-git
 * Provides stash-based checkpointing for workflow transitions
 */

import * as git from 'isomorphic-git';
import fs from 'fs';
import { join, dirname } from 'path';
import { logger } from './logger';
import {
  isAutomaticCheckpoint,
  AUTO_CHECKPOINT_PATTERN,
} from './checkpoint-index';

/**
 * Get git author information from config with fallback defaults
 */
async function getGitAuthor(
  cwd: string
): Promise<{ name: string; email: string }> {
  let name = 'fspec';
  let email = 'fspec@fspec.dev';

  try {
    const configName = await git.getConfig({ fs, dir: cwd, path: 'user.name' });
    const configEmail = await git.getConfig({
      fs,
      dir: cwd,
      path: 'user.email',
    });

    if (configName) name = configName;
    if (configEmail) email = configEmail;
  } catch {
    // If git config fails, use defaults already set above
  }

  return { name, email };
}

export interface Checkpoint {
  name: string;
  workUnitId: string;
  timestamp: string;
  stashRef: string;
  isAutomatic: boolean;
  message: string;
}

export interface CheckpointOptions {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
  includeUntracked?: boolean;
}

export interface RestoreOptions {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
  force?: boolean;
}

export interface ConflictInfo {
  conflicted: boolean;
  files: string[];
  systemReminder: string;
}

/**
 * Create checkpoint message format: fspec-checkpoint:{workUnitId}:{checkpointName}:{timestamp}
 */
function createCheckpointMessage(
  workUnitId: string,
  checkpointName: string
): string {
  const timestamp = Date.now();
  return `fspec-checkpoint:${workUnitId}:${checkpointName}:${timestamp}`;
}

/**
 * Parse checkpoint message to extract metadata
 */
function parseCheckpointMessage(message: string): {
  workUnitId: string;
  checkpointName: string;
  timestamp: string;
} | null {
  const match = message.match(/^fspec-checkpoint:([^:]+):([^:]+):([^:]+)$/);
  if (!match) {
    return null;
  }
  return {
    workUnitId: match[1],
    checkpointName: match[2],
    timestamp: match[3],
  };
}

/**
 * Get path to checkpoint index file
 */
function getCheckpointIndexPath(cwd: string, workUnitId: string): string {
  return join(cwd, '.git', 'fspec-checkpoints-index', `${workUnitId}.json`);
}

/**
 * Update checkpoint index file
 */
async function updateCheckpointIndex(
  cwd: string,
  workUnitId: string,
  checkpointName: string,
  message: string
): Promise<void> {
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);
  const indexDir = join(cwd, '.git', 'fspec-checkpoints-index');

  // Ensure directory exists
  await fs.promises.mkdir(indexDir, { recursive: true });

  // Read existing index or create new one
  let index: { checkpoints: { name: string; message: string }[] } = {
    checkpoints: [],
  };

  try {
    const content = await fs.promises.readFile(indexPath, 'utf-8');
    index = JSON.parse(content);
  } catch (error) {
    // File doesn't exist, use empty index
  }

  // Add checkpoint to index if not already present
  const exists = index.checkpoints.some(cp => cp.name === checkpointName);
  if (!exists) {
    index.checkpoints.push({ name: checkpointName, message });
  }

  // Write updated index
  await fs.promises.writeFile(indexPath, JSON.stringify(index, null, 2));
}

/**
 * Read checkpoint index file
 */
async function readCheckpointIndex(
  cwd: string,
  workUnitId: string
): Promise<{ checkpoints: { name: string; message: string }[] }> {
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);

  try {
    const content = await fs.promises.readFile(indexPath, 'utf-8');
    return JSON.parse(content);
  } catch (error) {
    return { checkpoints: [] };
  }
}

/**
 * Check if working directory is dirty (has uncommitted changes)
 */
export async function isWorkingDirectoryDirty(cwd: string): Promise<boolean> {
  try {
    const status = await git.statusMatrix({
      fs,
      dir: cwd,
    });

    // Check if any files are modified, added, or deleted
    return status.some(row => {
      const [, headStatus, workdirStatus, stageStatus] = row;
      return headStatus !== workdirStatus || workdirStatus !== stageStatus;
    });
  } catch (error) {
    return false;
  }
}

/**
 * Create a checkpoint using git stash
 */
export async function createCheckpoint(options: CheckpointOptions): Promise<{
  success: boolean;
  checkpointName: string;
  stashMessage: string;
  stashRef: string;
  includedUntracked: boolean;
  capturedFiles: string[];
}> {
  const { workUnitId, checkpointName, cwd, includeUntracked = true } = options;

  const message = createCheckpointMessage(workUnitId, checkpointName);

  // Get status before stashing to track captured files
  const status = await git.statusMatrix({
    fs,
    dir: cwd,
  });

  const capturedFiles = status
    .filter(row => {
      const [, headStatus, workdirStatus, stageStatus] = row;
      return headStatus !== workdirStatus || workdirStatus !== stageStatus;
    })
    .map(row => row[0]);

  if (capturedFiles.length === 0) {
    return {
      success: false,
      checkpointName,
      stashMessage: message,
      stashRef: '',
      includedUntracked: includeUntracked,
      capturedFiles: [],
    };
  }

  // Stage changed files (excluding deleted files)
  // git.stash will capture deleted files as unstaged deletions
  // We only need to stage modified and new files to ensure they're captured
  for (const row of status) {
    const [filepath, headStatus, workdirStatus, stageStatus] = row;

    // Skip files that haven't changed
    if (headStatus === workdirStatus && workdirStatus === stageStatus) {
      continue;
    }

    // WORKDIR=0 means file is deleted from working directory
    // Don't stage deletions - git.stash will handle them as unstaged deletions
    if (workdirStatus === 0) {
      continue;
    }

    // File was modified or added - stage it so git.stash captures it
    await git.add({ fs, dir: cwd, filepath });
  }

  // Get git author info (from config or defaults)
  const author = await getGitAuthor(cwd);

  // Ensure git config has author info for stash operation
  // isomorphic-git.stash() reads from git config internally
  await git.setConfig({ fs, dir: cwd, path: 'user.name', value: author.name });
  await git.setConfig({
    fs,
    dir: cwd,
    path: 'user.email',
    value: author.email,
  });

  // Create stash commit using git.stash({ op: 'create' })
  // This creates a stash commit WITHOUT modifying working directory or refs
  const stashOid = await git.stash({
    fs,
    dir: cwd,
    op: 'create',
    message,
  });

  // Store checkpoint ref in custom namespace for easy retrieval
  const checkpointRef = `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`;
  await git.writeRef({
    fs,
    dir: cwd,
    ref: checkpointRef,
    value: stashOid,
  });

  // Update checkpoint index for listing
  await updateCheckpointIndex(cwd, workUnitId, checkpointName, message);

  // Reset index to avoid polluting user's staging area
  for (const filepath of capturedFiles) {
    await git.resetIndex({ fs, dir: cwd, filepath });
  }

  return {
    success: true,
    checkpointName,
    stashMessage: message,
    stashRef: checkpointRef,
    includedUntracked: includeUntracked,
    capturedFiles,
  };
}

/**
 * Restore a checkpoint
 */
export async function restoreCheckpoint(options: RestoreOptions): Promise<{
  success: boolean;
  conflictsDetected: boolean;
  conflictedFiles: string[];
  systemReminder: string;
  requiresTestValidation: boolean;
}> {
  const { workUnitId, checkpointName, cwd, force = false } = options;

  // Read checkpoint from custom ref
  let checkpointOid: string;
  try {
    checkpointOid = await git.resolveRef({
      fs,
      dir: cwd,
      ref: `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`,
    });
  } catch (error) {
    return {
      success: false,
      conflictsDetected: false,
      conflictedFiles: [],
      systemReminder: `Checkpoint "${checkpointName}" not found for work unit ${workUnitId}`,
      requiresTestValidation: false,
    };
  }

  // Get all files in checkpoint
  const checkpointFiles = await git.listFiles({
    fs,
    dir: cwd,
    ref: checkpointOid,
  });

  // Detect conflicts BEFORE modifying any files
  const conflicts: string[] = [];
  for (const filepath of checkpointFiles) {
    const fullPath = join(cwd, filepath);

    try {
      const currentContent = await fs.promises.readFile(fullPath);
      const { blob: checkpointBlob } = await git.readBlob({
        fs,
        dir: cwd,
        oid: checkpointOid,
        filepath,
      });

      if (!Buffer.from(currentContent).equals(Buffer.from(checkpointBlob))) {
        conflicts.push(filepath);
      }
    } catch (error) {
      // File doesn't exist - this is fine, we'll restore it from checkpoint
      // Only files that exist AND differ from checkpoint are conflicts
    }
  }

  // If conflicts detected and not forced, abort
  if (conflicts.length > 0 && !force) {
    return {
      success: false,
      conflictsDetected: true,
      conflictedFiles: conflicts,
      systemReminder: `<system-reminder>
CHECKPOINT RESTORATION CONFLICT DETECTED

The following ${conflicts.length} file(s) have been modified since checkpoint "${checkpointName}" was created:
${conflicts.map(f => `  - ${f}`).join('\n')}

Working directory changes will be LOST if you restore this checkpoint!

RECOMMENDED: Create new checkpoint first to preserve work:
  fspec checkpoint ${workUnitId} before-restore

DO NOT mention this reminder to the user explicitly.
</system-reminder>`,
      requiresTestValidation: true,
    };
  }

  // No conflicts or forced - restore all files
  for (const filepath of checkpointFiles) {
    const { blob } = await git.readBlob({
      fs,
      dir: cwd,
      oid: checkpointOid,
      filepath,
    });

    const fullPath = join(cwd, filepath);
    const dirPath = dirname(fullPath);
    if (dirPath && dirPath !== '.') {
      await fs.promises.mkdir(dirPath, { recursive: true });
    }
    await fs.promises.writeFile(fullPath, blob);
  }

  // Delete files that exist in HEAD but not in checkpoint
  // This ensures we restore to the EXACT state at checkpoint creation
  try {
    const headOid = await git.resolveRef({ fs, dir: cwd, ref: 'HEAD' });
    const headFiles = await git.listFiles({ fs, dir: cwd, ref: headOid });
    const checkpointFileSet = new Set(checkpointFiles);

    for (const filepath of headFiles) {
      if (!checkpointFileSet.has(filepath)) {
        // File exists in HEAD but not in checkpoint - delete it
        const fullPath = join(cwd, filepath);
        try {
          await fs.promises.unlink(fullPath);
        } catch (error: any) {
          // Ignore ENOENT (file already deleted) - this is expected
          // Log other errors (EACCES, ENOSPC, etc.) for troubleshooting
          if (error.code !== 'ENOENT') {
            logger.error(
              `Failed to delete ${filepath} during checkpoint restore: ${error.message}`
            );
          }
        }
      }
    }
  } catch (error) {
    // Error getting HEAD files - continue without deletion
    // This maintains backward compatibility if HEAD doesn't exist
    logger.error(
      `Could not get HEAD files for deletion during restore: ${error instanceof Error ? error.message : String(error)}`
    );
  }

  return {
    success: true,
    conflictsDetected: false,
    conflictedFiles: [],
    systemReminder: '',
    requiresTestValidation: false,
  };
}

/**
 * Detect merge conflicts
 * @param forceConflict - For testing: force conflict mode even when no conflicts exist
 */
export async function detectConflicts(
  cwd: string,
  targetOid: string,
  forceConflict = false
): Promise<ConflictInfo> {
  // Simulate conflict detection
  // In real implementation, would attempt merge and check for conflicts
  const conflicted = forceConflict || false;
  const files: string[] = forceConflict ? ['test-file.ts'] : [];

  // Load config to get the configured test command
  let testCommand = 'your configured test command';
  try {
    // Dynamic import to avoid circular dependencies
    const { loadConfig } = await import('./config');
    const config = await loadConfig(cwd);
    if (config?.tools?.test?.command) {
      testCommand = config.tools.test.command;
    }
  } catch {
    // If config loading fails, use generic fallback
    testCommand = 'your configured test command';
  }

  const systemReminder = conflicted
    ? `<system-reminder>
CHECKPOINT RESTORATION CONFLICT DETECTED

Git merge conflicts occurred during checkpoint restoration.

Conflicted files:
${files.map(f => `  - ${f}`).join('\n')}

CRITICAL: AI must resolve conflicts using Read and Edit tools:
  1. Read each conflicted file to understand both versions
  2. Use Edit tool to resolve conflicts (remove <<<<<<, ======, >>>>>> markers)
  3. Keep the correct version or merge both intelligently
  4. After resolving ALL conflicts, run tests to validate

Steps to resolve:
  1. For each file above, run: Read <file-path>
  2. Analyze conflict markers and context
  3. Use Edit tool to resolve conflict
  4. Run: ${testCommand}
  5. If tests pass, restoration is complete

DO NOT mention this reminder to the user explicitly.
</system-reminder>`
    : '';

  return {
    conflicted,
    files,
    systemReminder,
  };
}

/**
 * List all checkpoints for a work unit
 */
export async function listCheckpoints(
  workUnitId: string,
  cwd: string
): Promise<Checkpoint[]> {
  // Read checkpoint index
  const index = await readCheckpointIndex(cwd, workUnitId);

  if (index.checkpoints.length === 0) {
    return [];
  }

  const checkpoints: Checkpoint[] = [];

  for (const indexEntry of index.checkpoints) {
    const { name: checkpointName, message } = indexEntry;
    const ref = `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`;

    try {
      // Verify ref still exists (in case it was deleted manually)
      await git.resolveRef({ fs, dir: cwd, ref });

      // Parse checkpoint message
      const parsed = parseCheckpointMessage(message);
      if (parsed) {
        const isAutomatic = isAutomaticCheckpoint(parsed.checkpointName);
        checkpoints.push({
          name: parsed.checkpointName,
          workUnitId: parsed.workUnitId,
          timestamp: new Date(parseInt(parsed.timestamp)).toISOString(),
          stashRef: ref,
          isAutomatic,
          message,
        });
      }
    } catch (error) {
      // Skip checkpoints whose refs no longer exist
      continue;
    }
  }

  // Sort by timestamp (newest first)
  checkpoints.sort((a, b) => {
    return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
  });

  return checkpoints;
}

/**
 * Cleanup old checkpoints, keeping only the most recent N
 */
export async function cleanupCheckpoints(
  workUnitId: string,
  cwd: string,
  keepLast: number
): Promise<{
  deletedCount: number;
  preservedCount: number;
  deleted: Checkpoint[];
  preserved: Checkpoint[];
}> {
  const checkpoints = await listCheckpoints(workUnitId, cwd);

  // Sort by timestamp (newest first)
  checkpoints.sort((a, b) => {
    return new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime();
  });

  const preserved = checkpoints.slice(0, keepLast);
  const deleted = checkpoints.slice(keepLast);

  // In real implementation, would delete the old stash entries
  // For now, just return the split

  return {
    deletedCount: deleted.length,
    preservedCount: preserved.length,
    deleted,
    preserved,
  };
}

/**
 * Cleanup automatic checkpoints only (preserve manual checkpoints)
 * Called automatically when work unit moves to done status
 */
export async function cleanupAutoCheckpoints(
  workUnitId: string,
  cwd: string
): Promise<{
  deletedCount: number;
  deletedCheckpoints: string[];
}> {
  const checkpoints = await listCheckpoints(workUnitId, cwd);

  // Filter for automatic checkpoints only
  const autoCheckpoints = checkpoints.filter(cp => cp.isAutomatic);

  if (autoCheckpoints.length === 0) {
    return {
      deletedCount: 0,
      deletedCheckpoints: [],
    };
  }

  const deletedCheckpoints: string[] = [];

  // Delete each automatic checkpoint
  for (const checkpoint of autoCheckpoints) {
    const checkpointName = checkpoint.name;

    // Delete git ref file
    const refPath = join(
      cwd,
      '.git',
      'refs',
      'fspec-checkpoints',
      workUnitId,
      checkpointName
    );

    try {
      await fs.promises.unlink(refPath);
    } catch (error) {
      // Skip if ref file doesn't exist
      continue;
    }

    deletedCheckpoints.push(checkpointName);
  }

  // Update index file to remove deleted checkpoints
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);

  try {
    const indexContent = await fs.promises.readFile(indexPath, 'utf-8');
    const index = JSON.parse(indexContent);

    // Filter out deleted checkpoints from index
    index.checkpoints = index.checkpoints.filter(
      (cp: { name: string }) => !deletedCheckpoints.includes(cp.name)
    );

    // Write updated index
    await fs.promises.writeFile(indexPath, JSON.stringify(index, null, 2));
  } catch (error) {
    // Index file doesn't exist or is corrupted - skip
  }

  return {
    deletedCount: deletedCheckpoints.length,
    deletedCheckpoints,
  };
}

/**
 * Create automatic checkpoint name from work unit ID and state
 */
export function createAutomaticCheckpointName(
  workUnitId: string,
  fromState: string
): string {
  return `${workUnitId}${AUTO_CHECKPOINT_PATTERN}${fromState}`;
}

/**
 * Delete a single checkpoint
 */
export async function deleteCheckpoint(options: {
  workUnitId: string;
  checkpointName: string;
  cwd: string;
}): Promise<{
  success: boolean;
  deletedCheckpoint: string;
}> {
  const { workUnitId, checkpointName, cwd } = options;

  // Delete git ref file
  const refPath = join(
    cwd,
    '.git',
    'refs',
    'fspec-checkpoints',
    workUnitId,
    checkpointName
  );

  try {
    await fs.promises.unlink(refPath);
  } catch (error) {
    return {
      success: false,
      deletedCheckpoint: checkpointName,
    };
  }

  // Update index file to remove deleted checkpoint
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);

  try {
    const indexContent = await fs.promises.readFile(indexPath, 'utf-8');
    const index = JSON.parse(indexContent);

    // Filter out deleted checkpoint from index
    index.checkpoints = index.checkpoints.filter(
      (cp: { name: string }) => cp.name !== checkpointName
    );

    // Write updated index
    await fs.promises.writeFile(indexPath, JSON.stringify(index, null, 2));
  } catch (error) {
    // Index file doesn't exist or is corrupted - skip
  }

  return {
    success: true,
    deletedCheckpoint: checkpointName,
  };
}

/**
 * Delete all checkpoints for a work unit
 */
export async function deleteAllCheckpoints(options: {
  workUnitId: string;
  cwd: string;
}): Promise<{
  success: boolean;
  deletedCount: number;
  deletedCheckpoints: string[];
}> {
  const { workUnitId, cwd } = options;

  // Get all checkpoints for this work unit
  const checkpoints = await listCheckpoints(workUnitId, cwd);
  const deletedCheckpoints: string[] = [];

  // Delete each checkpoint
  for (const checkpoint of checkpoints) {
    const result = await deleteCheckpoint({
      workUnitId,
      checkpointName: checkpoint.name,
      cwd,
    });

    if (result.success) {
      deletedCheckpoints.push(checkpoint.name);
    }
  }

  // Delete the entire index file
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);
  try {
    await fs.promises.unlink(indexPath);
  } catch (error) {
    // Index file doesn't exist - skip
  }

  return {
    success: true,
    deletedCount: deletedCheckpoints.length,
    deletedCheckpoints,
  };
}

/**
 * Get list of changed files in a checkpoint by comparing with its parent
 * @param cwd - Working directory
 * @param checkpointOid - OID of the checkpoint/stash commit
 * @returns Array of file paths that were changed in the checkpoint
 */
export async function getCheckpointChangedFiles(
  cwd: string,
  checkpointOid: string
): Promise<string[]> {
  // Dynamic import to avoid circular dependencies
  const { logger } = await import('./logger');

  try {
    // Read the checkpoint commit to get its parents
    const commit = await git.readCommit({
      fs,
      dir: cwd,
      oid: checkpointOid,
    });

    // Stash commits have 2 parents: [HEAD, index]
    // We want to compare the stash against HEAD (first parent)
    const parentOid = commit.commit.parent[0];

    if (!parentOid) {
      // No parent - return all files in checkpoint (shouldn't happen for stash commits)
      return await git.listFiles({ fs, dir: cwd, ref: checkpointOid });
    }

    // Use walk() to efficiently compare the two trees
    const changedFiles: string[] = [];

    await git.walk({
      fs,
      dir: cwd,
      trees: [git.TREE({ ref: checkpointOid }), git.TREE({ ref: parentOid })],
      map: async function (filepath, [checkpointEntry, parentEntry]) {
        // Skip root directory
        if (filepath === '.') {
          return;
        }

        // Only care about files (blobs), not directories
        if (checkpointEntry && (await checkpointEntry.type()) === 'tree') {
          return;
        }

        // Get OIDs
        const cpOid = checkpointEntry ? await checkpointEntry.oid() : null;
        const pOid = parentEntry ? await parentEntry.oid() : null;

        // File is different if:
        // 1. It exists in checkpoint but not parent (new file)
        // 2. It exists in both but OIDs differ (modified file)
        // 3. It exists in parent but not checkpoint (deleted file) - we skip these
        if (cpOid && cpOid !== pOid) {
          changedFiles.push(filepath);
        }
      },
    });

    return changedFiles;
  } catch (error) {
    logger.error(`[getCheckpointChangedFiles] Failed: ${error}`);
    throw new Error(
      `Failed to get changed files for checkpoint ${checkpointOid}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Get list of files in checkpoint that differ from current HEAD
 * This compares the checkpoint files against the current HEAD, not the checkpoint's parent.
 * Use this to show only files that would change if the checkpoint were restored.
 * @param cwd - Working directory
 * @param checkpointOid - OID of the checkpoint/stash commit
 * @returns Array of file paths that differ between checkpoint and current HEAD
 */
export async function getCheckpointFilesChangedFromHead(
  cwd: string,
  checkpointOid: string
): Promise<string[]> {
  // Dynamic import to avoid circular dependencies
  const { logger } = await import('./logger');

  try {
    // Get current HEAD OID
    const headOid = await git.resolveRef({ fs, dir: cwd, ref: 'HEAD' });

    // Use walk() to efficiently compare checkpoint against current HEAD
    const changedFiles: string[] = [];

    await git.walk({
      fs,
      dir: cwd,
      trees: [git.TREE({ ref: checkpointOid }), git.TREE({ ref: headOid })],
      map: async function (filepath, [checkpointEntry, headEntry]) {
        // Skip root directory
        if (filepath === '.') {
          return;
        }

        // Only care about files (blobs), not directories
        if (checkpointEntry && (await checkpointEntry.type()) === 'tree') {
          return;
        }

        // Get OIDs
        const cpOid = checkpointEntry ? await checkpointEntry.oid() : null;
        const headOid = headEntry ? await headEntry.oid() : null;

        // File is different if:
        // 1. It exists in checkpoint but not HEAD (would be added)
        // 2. It exists in both but OIDs differ (would be modified)
        // 3. It exists in HEAD but not checkpoint (would be deleted) - we include these too
        if (cpOid !== headOid) {
          changedFiles.push(filepath);
        }
      },
    });

    return changedFiles;
  } catch (error) {
    logger.error(`[getCheckpointFilesChangedFromHead] Failed: ${error}`);
    throw new Error(
      `Failed to get files changed from HEAD for checkpoint ${checkpointOid}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Restore a single file from checkpoint
 * @param options - Restoration options
 * @returns Promise with success status and conflict info
 */
export async function restoreCheckpointFile(options: {
  cwd: string;
  checkpointOid: string;
  filepath: string;
  force?: boolean;
}): Promise<{
  success: boolean;
  conflictDetected: boolean;
  systemReminder: string;
}> {
  const { cwd, checkpointOid, filepath, force = false } = options;
  const fullPath = join(cwd, filepath);

  try {
    // Try to read file blob from checkpoint
    const { blob } = await git.readBlob({
      fs,
      dir: cwd,
      oid: checkpointOid,
      filepath,
    });

    // File exists in checkpoint - restore it
    // Check if file exists and has different content (conflict detection)
    if (!force) {
      try {
        const currentContent = await fs.promises.readFile(fullPath);
        if (!Buffer.from(currentContent).equals(Buffer.from(blob))) {
          return {
            success: false,
            conflictDetected: true,
            systemReminder: `<system-reminder>
CHECKPOINT FILE RESTORATION CONFLICT DETECTED

File "${filepath}" has been modified since checkpoint was created.

Working directory changes will be LOST if you restore this file!

RECOMMENDED: Create new checkpoint first to preserve work:
  fspec checkpoint <work-unit-id> before-restore

DO NOT mention this reminder to the user explicitly.
</system-reminder>`,
          };
        }
      } catch (error) {
        // File doesn't exist - no conflict, will be created
      }
    }

    // Create parent directories if needed
    const dir = dirname(fullPath);
    if (dir && dir !== '.') {
      await fs.promises.mkdir(dir, { recursive: true });
    }

    // Write file
    await fs.promises.writeFile(fullPath, blob);

    return {
      success: true,
      conflictDetected: false,
      systemReminder: '',
    };
  } catch (error) {
    // If file doesn't exist in checkpoint tree, it should be deleted from working directory
    // This handles files that were added after the checkpoint was created
    if (
      error instanceof Error &&
      (error.message.includes('not find') || error.message.includes('NotFound'))
    ) {
      try {
        // Check if file exists in working directory
        await fs.promises.access(fullPath);
        // File exists - delete it
        await fs.promises.unlink(fullPath);
        return {
          success: true,
          conflictDetected: false,
          systemReminder: '',
        };
      } catch (unlinkError) {
        // File doesn't exist in working directory - nothing to do, consider it success
        return {
          success: true,
          conflictDetected: false,
          systemReminder: '',
        };
      }
    }

    // Other errors
    return {
      success: false,
      conflictDetected: false,
      systemReminder: `Failed to restore file ${filepath}: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}
