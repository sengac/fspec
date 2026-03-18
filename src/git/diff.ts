/**
 * Git diff operations using gitoxide (gix) via NAPI bindings
 *
 * Coverage:
 * - GIT-040: Replace diff-worker.ts with native Rust NAPI diff operations
 */

import {
  getFileDiff as napiGetFileDiff,
  getCheckpointFileDiff as napiGetCheckpointFileDiff,
} from '@sengac/codelet-napi';

/**
 * Get unified diff for a specific file
 * @param cwd - Working directory path
 * @param filepath - Relative path to file from cwd
 * @returns Unified diff string or null if no changes
 */
export function getFileDiff(cwd: string, filepath: string): string | null {
  try {
    const result = napiGetFileDiff(cwd, filepath);
    return result ?? null;
  } catch (error) {
    throw new Error(
      `Failed to get diff for ${filepath}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Get diff between a checkpoint file and HEAD
 * @param cwd - Working directory path
 * @param filepath - Relative path to file from cwd
 * @param checkpointRef - Git ref for the checkpoint (e.g. "refs/fspec-checkpoints/WORK-001/baseline")
 * @returns Unified diff string or null if no changes
 */
export function getCheckpointFileDiff(
  cwd: string,
  filepath: string,
  checkpointRef: string
): string | null {
  try {
    const result = napiGetCheckpointFileDiff(cwd, filepath, checkpointRef);
    return result ?? null;
  } catch (error) {
    throw new Error(
      `Failed to get checkpoint diff for ${filepath}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}
