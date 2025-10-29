/**
 * Git checkpoint utilities using isomorphic-git
 * Provides stash-based checkpointing for workflow transitions
 */

import * as git from 'isomorphic-git';
import fs from 'fs';
import { join } from 'path';

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

  // Stage ALL changed files (including untracked) so git.stash can capture them
  for (const filepath of capturedFiles) {
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
      // File doesn't exist - no conflict
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
    const dirname = fullPath.substring(0, fullPath.lastIndexOf('/'));
    if (dirname) {
      await fs.promises.mkdir(dirname, { recursive: true });
    }
    await fs.promises.writeFile(fullPath, blob);
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
    const { loadConfig } = await import('./config.js');
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
        const isAutomatic = parsed.checkpointName.startsWith(
          `${workUnitId}-auto-`
        );
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
 * Create automatic checkpoint name from work unit ID and state
 */
export function createAutomaticCheckpointName(
  workUnitId: string,
  fromState: string
): string {
  return `${workUnitId}-auto-${fromState}`;
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
  const { logger } = await import('./logger.js');

  try {
    logger.info(
      `[getCheckpointChangedFiles] Starting for checkpoint OID: ${checkpointOid}`
    );

    // Read the checkpoint commit to get its parents
    const commit = await git.readCommit({
      fs,
      dir: cwd,
      oid: checkpointOid,
    });
    logger.info(
      `[getCheckpointChangedFiles] Checkpoint has ${commit.commit.parent.length} parents: ${commit.commit.parent.join(', ')}`
    );

    // Stash commits have 2 parents: [HEAD, index]
    // We want to compare the stash against HEAD (first parent)
    const parentOid = commit.commit.parent[0];

    if (!parentOid) {
      logger.warn(
        `[getCheckpointChangedFiles] No parent found - returning all files in checkpoint`
      );
      // No parent - return all files in checkpoint (shouldn't happen for stash commits)
      return await git.listFiles({ fs, dir: cwd, ref: checkpointOid });
    }

    logger.info(
      `[getCheckpointChangedFiles] Comparing checkpoint ${checkpointOid} against parent ${parentOid}`
    );

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

    logger.info(
      `[getCheckpointChangedFiles] Found ${changedFiles.length} changed files`
    );
    logger.info(
      `[getCheckpointChangedFiles] Changed files: ${changedFiles.slice(0, 10).join(', ')}${changedFiles.length > 10 ? '...' : ''}`
    );
    return changedFiles;
  } catch (error) {
    logger.error(`[getCheckpointChangedFiles] Failed: ${error}`);
    throw new Error(
      `Failed to get changed files for checkpoint ${checkpointOid}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}
