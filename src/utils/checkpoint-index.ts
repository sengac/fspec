/**
 * Checkpoint Index Utilities
 *
 * Pure functions for reading checkpoint index data from the filesystem.
 * These are designed to be composable, handle missing directories gracefully,
 * and follow SOLID/DRY principles.
 *
 * The checkpoint index directory stores JSON files at:
 *   .git/fspec-checkpoints-index/{workUnitId}.json
 *
 * Each index file has the format:
 *   { checkpoints: [{ name: string, message: string }] }
 *
 * Coverage:
 * - TUI-016: Checkpoint counts in TUI
 * - GIT-004: Checkpoint viewer functionality
 */

import { join } from 'path';
import { promises as fsPromises } from 'fs';
import fs from 'fs';

/**
 * Checkpoint counts (manual vs automatic)
 */
export interface CheckpointCounts {
  manual: number;
  auto: number;
}

/**
 * Single checkpoint entry in an index file
 */
export interface CheckpointIndexEntry {
  name: string;
  message: string;
}

/**
 * Structure of a checkpoint index file
 */
export interface CheckpointIndex {
  checkpoints: CheckpointIndexEntry[];
}

/**
 * Pattern used to identify automatic checkpoints.
 * Automatic checkpoints have names like: {workUnitId}-auto-{state}
 * Example: "TUI-001-auto-testing", "BUG-002-auto-specifying"
 */
export const AUTO_CHECKPOINT_PATTERN = '-auto-';

/**
 * Check if a checkpoint name indicates an automatic checkpoint.
 *
 * Automatic checkpoints are created automatically during state transitions
 * and follow the naming pattern: {workUnitId}-auto-{state}
 *
 * @param checkpointName - The checkpoint name to check
 * @returns true if this is an automatic checkpoint
 */
export function isAutomaticCheckpoint(checkpointName: string): boolean {
  return checkpointName.includes(AUTO_CHECKPOINT_PATTERN);
}

/**
 * Parse an automatic checkpoint name into its component parts.
 *
 * @param checkpointName - The checkpoint name (e.g., "TUI-001-auto-testing")
 * @returns Object with workUnitId and state, or null if not an automatic checkpoint
 */
export function parseAutomaticCheckpointName(checkpointName: string): {
  workUnitId: string;
  state: string;
} | null {
  if (!isAutomaticCheckpoint(checkpointName)) {
    return null;
  }

  const parts = checkpointName.split(AUTO_CHECKPOINT_PATTERN);
  if (parts.length !== 2) {
    return null;
  }

  return {
    workUnitId: parts[0],
    state: parts[1],
  };
}

/**
 * Get the checkpoint index directory path
 *
 * @param cwd - Working directory (project root with .git folder)
 * @returns Absolute path to checkpoint index directory
 */
export function getCheckpointIndexDir(cwd: string): string {
  return join(cwd, '.git', 'fspec-checkpoints-index');
}

/**
 * Check if checkpoint index directory exists (synchronous)
 *
 * Use this for quick existence checks before more expensive operations.
 *
 * @param cwd - Working directory
 * @returns true if the directory exists
 */
export function checkpointIndexDirExists(cwd: string): boolean {
  const indexDir = getCheckpointIndexDir(cwd);
  return fs.existsSync(indexDir);
}

/**
 * Safely list all checkpoint index files (JSON files in the index directory)
 *
 * This function gracefully handles the case where the directory doesn't exist,
 * which is expected for:
 * - New projects that haven't created any checkpoints
 * - Test environments with fresh temporary directories
 *
 * @param cwd - Working directory
 * @returns Array of JSON filenames (e.g., ['WORK-001.json', 'BUG-002.json'])
 * @throws Only on unexpected errors (permission denied, I/O errors, etc.)
 */
export async function listCheckpointIndexFiles(cwd: string): Promise<string[]> {
  const indexDir = getCheckpointIndexDir(cwd);

  try {
    const files = await fsPromises.readdir(indexDir);
    return files.filter(f => f.endsWith('.json'));
  } catch (error: unknown) {
    // ENOENT = directory doesn't exist, which is expected for new projects
    if (error instanceof Error && 'code' in error) {
      const errnoError = error as Error & { code: string };
      if (errnoError.code === 'ENOENT') {
        return [];
      }
    }
    // Re-throw unexpected errors (permission denied, disk errors, etc.)
    throw error;
  }
}

/**
 * Read a single checkpoint index file for a work unit
 *
 * Returns empty checkpoint list if:
 * - File doesn't exist (work unit has no checkpoints)
 * - File is corrupted/invalid JSON
 *
 * @param cwd - Working directory
 * @param workUnitId - Work unit ID (e.g., 'WORK-001')
 * @returns Checkpoint index with list of checkpoints
 */
export async function readCheckpointIndexFile(
  cwd: string,
  workUnitId: string
): Promise<CheckpointIndex> {
  const indexDir = getCheckpointIndexDir(cwd);
  const filePath = join(indexDir, `${workUnitId}.json`);

  try {
    const content = await fsPromises.readFile(filePath, 'utf-8');
    const parsed = JSON.parse(content) as CheckpointIndex;
    // Ensure checkpoints array exists
    return {
      checkpoints: Array.isArray(parsed.checkpoints) ? parsed.checkpoints : [],
    };
  } catch {
    // File doesn't exist or is corrupted - return empty list
    return { checkpoints: [] };
  }
}

/**
 * Count manual vs automatic checkpoints across all work units
 *
 * This function gracefully handles:
 * - Non-existent checkpoint directory (returns {manual: 0, auto: 0})
 * - Corrupted index files (skipped silently)
 *
 * @param cwd - Working directory
 * @returns Counts of manual and automatic checkpoints
 */
export async function countCheckpoints(cwd: string): Promise<CheckpointCounts> {
  const jsonFiles = await listCheckpointIndexFiles(cwd);

  let manual = 0;
  let auto = 0;

  for (const jsonFile of jsonFiles) {
    // Extract work unit ID from filename (e.g., 'WORK-001.json' -> 'WORK-001')
    const workUnitId = jsonFile.replace('.json', '');

    // Use readCheckpointIndexFile for consistent error handling (DRY)
    const index = await readCheckpointIndexFile(cwd, workUnitId);

    for (const checkpoint of index.checkpoints) {
      if (isAutomaticCheckpoint(checkpoint.name)) {
        auto++;
      } else {
        manual++;
      }
    }
  }

  return { manual, auto };
}
