/**
 * Git status operations using isomorphic-git
 *
 * This module provides a semantic abstraction over isomorphic-git's statusMatrix,
 * hiding implementation details and providing TypeScript-friendly types.
 *
 * Uses isomorphic-git instead of git CLI for:
 * - Zero external dependencies (no git binary required)
 * - Cross-platform consistency
 * - Bundlable as single executable
 * - Testable with mock filesystems (memfs)
 */

import git from 'isomorphic-git';
import fsNode from 'fs';
import { join } from 'path';
import type { StatusRow } from 'isomorphic-git';
import type { IFs } from 'memfs';

/**
 * Semantic file status with boolean flags
 * Hides isomorphic-git's raw StatusRow format [filepath, HEAD, WORKDIR, STAGE]
 */
export interface FileStatus {
  filepath: string;
  /** File is staged (differs from HEAD commit) */
  staged: boolean;
  /** File has unstaged changes (working directory differs from staging area, but is not untracked) */
  hasUnstagedChanges: boolean;
  /** File is untracked (not in HEAD and not staged) */
  untracked: boolean;
}

/**
 * File change type following git conventions
 * A = Added (new file), M = Modified, D = Deleted, R = Renamed
 */
export type ChangeType = 'A' | 'M' | 'D' | 'R';

/**
 * File status with change type information
 * Used for displaying git status indicators in TUI
 */
export interface FileStatusWithChangeType {
  filepath: string;
  /** Change type: A (added), M (modified), D (deleted), R (renamed) */
  changeType: ChangeType;
  /** Whether the change is staged */
  staged: boolean;
}

/**
 * Configuration options for git operations
 */
export interface GitStatusOptions {
  /** If true, throw errors instead of returning empty arrays (default: false) */
  strict?: boolean;
  /**
   * Custom filesystem implementation (for testing with memfs)
   * Uses IFs interface from memfs for type safety
   */
  fs?: IFs;
}

/**
 * Check if directory is a git repository
 * @param dir - Directory to check
 * @param fs - Filesystem implementation
 * @returns true if .git directory exists
 */
function isGitRepository(dir: string, fs: IFs): boolean {
  try {
    const gitDir = join(dir, '.git');
    const stats = fs.statSync(gitDir);
    return stats.isDirectory();
  } catch {
    return false;
  }
}

/**
 * Get status matrix from isomorphic-git
 * Internal helper that wraps git.statusMatrix with error handling
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Status matrix or empty array on error (if strict=false)
 */
async function getStatusMatrix(
  dir: string,
  options?: GitStatusOptions
): Promise<StatusRow[]> {
  const fs = options?.fs || fsNode;

  // In strict mode, validate that we're in a git repository
  if (options?.strict && !isGitRepository(dir, fs)) {
    throw new Error(`Not a git repository: ${dir}`);
  }

  try {
    // NOTE: Added cache:false to force re-reading file content
    // This fixes memfs integration where file modifications weren't being detected
    // because isomorphic-git was relying on mtime which memfs doesn't update properly
    const matrix = await git.statusMatrix({ fs, dir, cache: {} as any });
    return matrix;
  } catch (error: unknown) {
    if (options?.strict) {
      throw error;
    }
    // Non-strict mode: return empty array (silent failure)
    return [];
  }
}

/**
 * Determine change type from status matrix values
 *
 * Status matrix format: [filepath, HEAD, WORKDIR, STAGE]
 * - HEAD: 0 = absent, 1 = present
 * - WORKDIR: 0 = absent, 1 = present, 2 = modified
 * - STAGE: 0 = absent, 1 = unmodified, 2 = modified, 3 = added
 *
 * @param head - HEAD status (0 or 1)
 * @param workdir - WORKDIR status (0, 1, or 2)
 * @param stage - STAGE status (0, 1, 2, or 3)
 * @returns Change type: A (added), M (modified), D (deleted), R (renamed)
 */
function getChangeType(
  head: number,
  workdir: number,
  stage: number
): ChangeType {
  // Deleted: file was in HEAD but no longer in WORKDIR
  if (head === 1 && workdir === 0) {
    return 'D';
  }

  // Added: file not in HEAD but present in WORKDIR/STAGE
  if (head === 0 && (workdir > 0 || stage > 0)) {
    return 'A';
  }

  // Modified: file exists in both HEAD and WORKDIR, with changes
  if (head === 1 && workdir === 2) {
    return 'M';
  }

  // Default to modified for other cases (e.g., staged modifications)
  return 'M';
}

/**
 * Get list of staged files
 *
 * Staged files are files that have been added to the index (git add).
 * These are ready to be committed.
 *
 * Status matrix logic: STAGE !== HEAD
 * - If STAGE differs from HEAD, the file has staged changes
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Array of staged file paths
 *
 * @example
 * ```typescript
 * const staged = await getStagedFiles('/repo');
 * // ['src/index.ts', 'README.md']
 * ```
 */
export async function getStagedFiles(
  dir: string,
  options?: GitStatusOptions
): Promise<string[]> {
  const matrix = await getStatusMatrix(dir, options);

  return matrix
    .filter(([, head, , stage]) => {
      // Staged: STAGE !== HEAD
      return stage !== head;
    })
    .map(([filepath]) => filepath);
}

/**
 * Get list of unstaged modified files
 *
 * Unstaged files are files that have been modified in the working directory
 * but have not been staged (git add).
 *
 * Status matrix logic:
 * - Modified files: WORKDIR === 2 (file is modified)
 * - Deleted files: WORKDIR === 0 && HEAD === 1 (file deleted from workdir)
 * - Must not be untracked: !(HEAD === 0 && STAGE === 0)
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Array of unstaged file paths
 *
 * @example
 * ```typescript
 * const unstaged = await getUnstagedFiles('/repo');
 * // ['src/utils.ts']
 * ```
 */
export async function getUnstagedFiles(
  dir: string,
  options?: GitStatusOptions
): Promise<string[]> {
  const matrix = await getStatusMatrix(dir, options);

  return matrix
    .filter(([, head, workdir, stage]) => {
      // Untracked files: not in HEAD and not staged
      const isUntracked = head === 0 && stage === 0;

      // Unstaged changes: working directory differs from staging area
      const hasUnstagedChanges = workdir !== stage;

      // Include files that have unstaged changes but are not untracked
      return hasUnstagedChanges && !isUntracked;
    })
    .map(([filepath]) => filepath);
}

/**
 * Get list of untracked files
 *
 * Untracked files are files that exist in the working directory but are
 * not tracked by git (never added, not in .git/index).
 *
 * Status matrix logic: HEAD === 0 && STAGE === 0
 * - HEAD === 0: File not in HEAD commit
 * - STAGE === 0: File not in staging area
 *
 * Note: isomorphic-git respects .gitignore by default, so ignored files
 * will not appear in the status matrix.
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Array of untracked file paths
 *
 * @example
 * ```typescript
 * const untracked = await getUntrackedFiles('/repo');
 * // ['newfile.txt', 'src/draft.ts']
 * ```
 */
export async function getUntrackedFiles(
  dir: string,
  options?: GitStatusOptions
): Promise<string[]> {
  const matrix = await getStatusMatrix(dir, options);

  return matrix
    .filter(([, head, , stage]) => {
      // Untracked: not in HEAD and not staged
      return head === 0 && stage === 0;
    })
    .map(([filepath]) => filepath);
}

/**
 * Get status for a specific file
 *
 * Returns semantic FileStatus object with boolean flags instead of
 * raw status matrix values.
 *
 * Status matrix values:
 * - HEAD: 0 = absent, 1 = present
 * - WORKDIR: 0 = absent, 1 = present, 2 = modified
 * - STAGE: 0 = absent, 1 = unmodified, 2 = modified, 3 = added
 *
 * @param dir - Repository directory
 * @param filepath - Path to file (relative to repository root)
 * @param options - Configuration options
 * @returns FileStatus object or null if file not found
 *
 * @example
 * ```typescript
 * const status = await getFileStatus('/repo', 'src/index.ts');
 * // { filepath: 'src/index.ts', staged: false, hasUnstagedChanges: true, untracked: false }
 * ```
 */
/**
 * Get current branch name
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Current branch name or undefined if not in a git repository
 */
export async function getCurrentBranch(
  dir: string,
  options?: GitStatusOptions
): Promise<string | undefined> {
  const fs = options?.fs || fsNode;

  if (!isGitRepository(dir, fs)) {
    return undefined;
  }

  try {
    const branch = await git.currentBranch({ fs, dir });
    return branch || undefined;
  } catch (error: unknown) {
    if (options?.strict) {
      throw error;
    }
    return undefined;
  }
}

/**
 * Get git status summary
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Array of file statuses
 */
export async function getGitStatus(
  dir: string,
  options?: GitStatusOptions
): Promise<FileStatus[]> {
  const staged = await getStagedFiles(dir, options);
  const unstaged = await getUnstagedFiles(dir, options);
  const untracked = await getUntrackedFiles(dir, options);

  return [...staged, ...unstaged, ...untracked];
}

export async function getFileStatus(
  dir: string,
  filepath: string,
  options?: GitStatusOptions
): Promise<FileStatus | null> {
  const matrix = await getStatusMatrix(dir, options);

  // Find the row for this specific file
  const row = matrix.find(([f]) => f === filepath);

  if (!row) {
    return null;
  }

  const [file, head, workdir, stage] = row;

  // Transform raw status matrix to semantic booleans
  return {
    filepath: file,
    staged: stage !== head, // Staged if STAGE differs from HEAD
    hasUnstagedChanges: workdir === 2 && stage === 1, // Modified but not staged
    untracked: head === 0 && stage === 0, // Not in HEAD and not staged
  };
}

/**
 * Get list of staged files with change type information
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Array of FileStatusWithChangeType for staged files
 */
export async function getStagedFilesWithChangeType(
  dir: string,
  options?: GitStatusOptions
): Promise<FileStatusWithChangeType[]> {
  const matrix = await getStatusMatrix(dir, options);

  return matrix
    .filter(([, head, , stage]) => {
      // Staged: STAGE !== HEAD
      return stage !== head;
    })
    .map(([filepath, head, workdir, stage]) => ({
      filepath,
      changeType: getChangeType(head, workdir, stage),
      staged: true,
    }));
}

/**
 * Get list of unstaged files with change type information
 *
 * @param dir - Repository directory
 * @param options - Configuration options
 * @returns Array of FileStatusWithChangeType for unstaged files
 */
export async function getUnstagedFilesWithChangeType(
  dir: string,
  options?: GitStatusOptions
): Promise<FileStatusWithChangeType[]> {
  const matrix = await getStatusMatrix(dir, options);

  return matrix
    .filter(([, head, workdir, stage]) => {
      // Untracked files: not in HEAD and not staged
      const isUntracked = head === 0 && stage === 0;

      // Unstaged changes: working directory differs from staging area
      const hasUnstagedChanges = workdir !== stage;

      // Include files that have unstaged changes but are not untracked
      return hasUnstagedChanges && !isUntracked;
    })
    .map(([filepath, head, workdir, stage]) => ({
      filepath,
      changeType: getChangeType(head, workdir, stage),
      staged: false,
    }));
}
