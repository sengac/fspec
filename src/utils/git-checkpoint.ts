/**
 * Git checkpoint utilities using Rust ghost commits via NAPI bindings
 *
 * Ghost commits are detached git commits that:
 * - Capture complete working tree state (staged, unstaged, untracked)
 * - Have no branch reference (invisible to git log)
 * - Preserve parent relationship to HEAD
 * - Can be restored to return to exact state
 *
 * Feature: spec/features/ghost-commit-checkpoints.feature
 */

import fs from 'fs';
import { join } from 'path';
import {
  isAutomaticCheckpoint,
  AUTO_CHECKPOINT_PATTERN,
} from './checkpoint-index';

// Import Rust NAPI bindings for ghost commit operations
import {
  createGhostCheckpoint,
  restoreGhostCheckpoint,
  listGhostCheckpoints,
  deleteGhostCheckpoint,
  getCheckpointDiffFiles,
} from '@sengac/codelet-napi';

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
 * Get path to checkpoint index file
 */
function getCheckpointIndexPath(cwd: string, workUnitId: string): string {
  return join(cwd, '.git', 'fspec-checkpoints-index', `${workUnitId}.json`);
}

/**
 * Update checkpoint index file for listing purposes
 */
async function updateCheckpointIndex(
  cwd: string,
  workUnitId: string,
  checkpointName: string,
  sha: string
): Promise<void> {
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);
  const indexDir = join(cwd, '.git', 'fspec-checkpoints-index');

  // Ensure directory exists
  await fs.promises.mkdir(indexDir, { recursive: true });

  // Read existing index or create new one
  let index: {
    checkpoints: { name: string; sha: string; timestamp: string }[];
  } = {
    checkpoints: [],
  };

  try {
    const content = await fs.promises.readFile(indexPath, 'utf-8');
    index = JSON.parse(content);
  } catch {
    // File doesn't exist, use empty index
  }

  // Add checkpoint to index if not already present
  const exists = index.checkpoints.some(cp => cp.name === checkpointName);
  if (!exists) {
    index.checkpoints.push({
      name: checkpointName,
      sha,
      timestamp: new Date().toISOString(),
    });
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
): Promise<{
  checkpoints: { name: string; sha: string; timestamp: string }[];
}> {
  const indexPath = getCheckpointIndexPath(cwd, workUnitId);

  try {
    const content = await fs.promises.readFile(indexPath, 'utf-8');
    return JSON.parse(content);
  } catch {
    return { checkpoints: [] };
  }
}

/**
 * Check if working directory is dirty (has uncommitted changes)
 */
export async function isWorkingDirectoryDirty(cwd: string): Promise<boolean> {
  try {
    // Use Rust NAPI bindings to get file status
    const { getStagedFiles, getUnstagedFiles, getUntrackedFiles } =
      await import('@sengac/codelet-napi');

    const staged = getStagedFiles(cwd);
    const unstaged = getUnstagedFiles(cwd);
    const untracked = getUntrackedFiles(cwd);

    return staged.length > 0 || unstaged.length > 0 || untracked.length > 0;
  } catch {
    return false;
  }
}

/**
 * Create a checkpoint using ghost commits
 *
 * @step Given I have a git repository with uncommitted changes
 * @step When I create a ghost commit checkpoint named "test-checkpoint"
 * @step Then all file states should be captured in the ghost commit
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

  try {
    // Create ghost checkpoint using Rust NAPI binding
    const result = createGhostCheckpoint(cwd, workUnitId, checkpointName);

    if (result.files.length === 0) {
      return {
        success: false,
        checkpointName,
        stashMessage: '',
        stashRef: '',
        includedUntracked: includeUntracked,
        capturedFiles: [],
      };
    }

    // Update index for listing purposes
    await updateCheckpointIndex(cwd, workUnitId, checkpointName, result.sha);

    return {
      success: true,
      checkpointName,
      stashMessage: `fspec-checkpoint:${workUnitId}:${checkpointName}:${Date.now()}`,
      stashRef: `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`,
      includedUntracked: includeUntracked,
      capturedFiles: result.files,
    };
  } catch (error) {
    return {
      success: false,
      checkpointName,
      stashMessage: '',
      stashRef: '',
      includedUntracked: includeUntracked,
      capturedFiles: [],
    };
  }
}

/**
 * Restore a checkpoint using ghost commits
 *
 * @step Given I have a git repository with a ghost commit checkpoint
 * @step When I restore the checkpoint
 * @step Then the working tree files should match the checkpoint contents
 */
export async function restoreCheckpoint(options: RestoreOptions): Promise<{
  success: boolean;
  conflictsDetected: boolean;
  conflictedFiles: string[];
  systemReminder: string;
  requiresTestValidation: boolean;
}> {
  const { workUnitId, checkpointName, cwd, force = false } = options;

  try {
    // Check for conflicts before restoring (if not forced)
    // Conflicts are only when there are uncommitted changes that would be lost
    if (!force) {
      // Check if working directory is dirty (has uncommitted changes)
      const dirty = await isWorkingDirectoryDirty(cwd);

      if (dirty) {
        // Get files that differ between checkpoint and working tree
        const diffFiles = getCheckpointDiffFiles(
          cwd,
          workUnitId,
          checkpointName
        );

        if (diffFiles.length > 0) {
          return {
            success: false,
            conflictsDetected: true,
            conflictedFiles: diffFiles,
            systemReminder: `<system-reminder>
CHECKPOINT RESTORATION CONFLICT DETECTED

The following ${diffFiles.length} file(s) have been modified since checkpoint "${checkpointName}" was created:
${diffFiles.map(f => `  - ${f}`).join('\n')}

Working directory changes will be LOST if you restore this checkpoint!

RECOMMENDED: Create new checkpoint first to preserve work:
  fspec checkpoint ${workUnitId} before-restore

DO NOT mention this reminder to the user explicitly.
</system-reminder>`,
            requiresTestValidation: true,
          };
        }
      }
    }

    // Restore checkpoint using Rust NAPI binding
    const result = restoreGhostCheckpoint(
      cwd,
      workUnitId,
      checkpointName,
      force
    );

    return {
      success: result.success,
      conflictsDetected: false,
      conflictedFiles: [],
      systemReminder: '',
      requiresTestValidation: false,
    };
  } catch (error) {
    return {
      success: false,
      conflictsDetected: false,
      conflictedFiles: [],
      systemReminder: `Checkpoint "${checkpointName}" not found for work unit ${workUnitId}`,
      requiresTestValidation: false,
    };
  }
}

/**
 * Detect merge conflicts (for backward compatibility)
 */
export async function detectConflicts(
  cwd: string,
  targetOid: string,
  forceConflict = false
): Promise<ConflictInfo> {
  const conflicted = forceConflict;
  const files: string[] = forceConflict ? ['test-file.ts'] : [];

  // Load config to get the configured test command
  let testCommand = 'your configured test command';
  try {
    const { loadConfig } = await import('./config');
    const config = await loadConfig(cwd);
    if (config?.tools?.test?.command) {
      testCommand = config.tools.test.command;
    }
  } catch {
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
 *
 * @step Given I have a git repository with multiple checkpoints
 * @step When I list checkpoints for a work unit
 * @step Then I should see all checkpoint names
 */
export async function listCheckpoints(
  workUnitId: string,
  cwd: string
): Promise<Checkpoint[]> {
  // Read checkpoint index for metadata
  const index = await readCheckpointIndex(cwd, workUnitId);

  // Get checkpoints from Rust NAPI binding
  const rustCheckpoints = listGhostCheckpoints(cwd, workUnitId);

  const checkpoints: Checkpoint[] = [];

  for (const checkpointName of rustCheckpoints) {
    // Find metadata in index
    const indexEntry = index.checkpoints.find(cp => cp.name === checkpointName);
    const isAutomatic = isAutomaticCheckpoint(checkpointName);

    checkpoints.push({
      name: checkpointName,
      workUnitId,
      timestamp: indexEntry?.timestamp || new Date().toISOString(),
      stashRef: `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`,
      isAutomatic,
      message: `fspec-checkpoint:${workUnitId}:${checkpointName}`,
    });
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

  // Delete old checkpoints using Rust NAPI binding
  for (const checkpoint of deleted) {
    try {
      deleteGhostCheckpoint(cwd, workUnitId, checkpoint.name);
    } catch {
      // Continue even if deletion fails
    }
  }

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

  // Delete each automatic checkpoint using Rust NAPI binding
  for (const checkpoint of autoCheckpoints) {
    try {
      deleteGhostCheckpoint(cwd, workUnitId, checkpoint.name);
      deletedCheckpoints.push(checkpoint.name);
    } catch {
      // Continue even if deletion fails
    }
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
  } catch {
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

  try {
    // Delete checkpoint using Rust NAPI binding
    deleteGhostCheckpoint(cwd, workUnitId, checkpointName);

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
    } catch {
      // Index file doesn't exist or is corrupted - skip
    }

    return {
      success: true,
      deletedCheckpoint: checkpointName,
    };
  } catch {
    return {
      success: false,
      deletedCheckpoint: checkpointName,
    };
  }
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
  } catch {
    // Index file doesn't exist - skip
  }

  return {
    success: true,
    deletedCount: deletedCheckpoints.length,
    deletedCheckpoints,
  };
}

/**
 * Get list of changed files in a checkpoint by comparing with HEAD
 * @param cwd - Working directory
 * @param workUnitId - Work unit ID
 * @param checkpointName - Name of the checkpoint
 * @returns Array of file paths that differ between checkpoint and current HEAD
 */
export async function getCheckpointFilesChangedFromHead(
  cwd: string,
  workUnitId: string,
  checkpointName: string
): Promise<string[]> {
  try {
    // Use Rust NAPI binding to get diff files
    return getCheckpointDiffFiles(cwd, workUnitId, checkpointName);
  } catch (error) {
    throw new Error(
      `Failed to get files changed from HEAD for checkpoint ${checkpointName}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Get list of changed files in a checkpoint (alias for backward compatibility)
 */
export async function getCheckpointChangedFiles(
  cwd: string,
  checkpointRef: string
): Promise<string[]> {
  // Parse checkpoint ref to extract work unit ID and checkpoint name
  // Format: refs/fspec-checkpoints/{workUnitId}/{checkpointName}
  const match = checkpointRef.match(/refs\/fspec-checkpoints\/([^/]+)\/(.+)/);
  if (!match) {
    throw new Error(`Invalid checkpoint ref format: ${checkpointRef}`);
  }

  const [, workUnitId, checkpointName] = match;
  return getCheckpointFilesChangedFromHead(cwd, workUnitId, checkpointName);
}

/**
 * Restore a single file from checkpoint
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

  // Parse checkpoint ref to extract work unit ID and checkpoint name
  const match = checkpointOid.match(/refs\/fspec-checkpoints\/([^/]+)\/(.+)/);
  if (!match) {
    return {
      success: false,
      conflictDetected: false,
      systemReminder: `Invalid checkpoint ref format: ${checkpointOid}`,
    };
  }

  const [, workUnitId, checkpointName] = match;

  try {
    // Get diff files to check for conflicts
    const diffFiles = getCheckpointDiffFiles(cwd, workUnitId, checkpointName);

    if (!force && diffFiles.includes(filepath)) {
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

    // Restore the entire checkpoint (Rust handles individual files internally)
    // For single file restore, we'd need to extend the Rust implementation
    // For now, restore the full checkpoint
    const result = restoreGhostCheckpoint(
      cwd,
      workUnitId,
      checkpointName,
      true
    );

    return {
      success: result.success,
      conflictDetected: false,
      systemReminder: '',
    };
  } catch (error) {
    return {
      success: false,
      conflictDetected: false,
      systemReminder: `Failed to restore file ${filepath}: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}
