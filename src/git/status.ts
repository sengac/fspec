/**
 * Git status operations using gitoxide (gix) via NAPI bindings
 *
 * This module provides a semantic abstraction over git status operations,
 * hiding implementation details and providing TypeScript-friendly types.
 *
 * Uses gitoxide (pure Rust) via NAPI for:
 * - Native performance
 * - Zero external dependencies (no git binary required)
 * - Cross-platform consistency
 * - Bundlable as single executable
 */

import {
  getStagedFiles as napiGetStagedFiles,
  getUnstagedFiles as napiGetUnstagedFiles,
  getUntrackedFiles as napiGetUntrackedFiles,
  getCurrentBranch as napiGetCurrentBranch,
} from '@sengac/codelet-napi';
import { execSync } from 'child_process';
import { join } from 'path';
import { existsSync, statSync } from 'fs';

/**
 * Semantic file status with boolean flags
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
 * @deprecated The fs option is no longer supported with gitoxide backend
 */
export interface GitStatusOptions {
  /** If true, throw errors instead of returning empty arrays (default: false) */
  strict?: boolean;
  /**
   * @deprecated Custom filesystem not supported with gitoxide backend.
   * This option is ignored. Tests should use real temporary directories.
   */
  fs?: unknown;
}

/**
 * Check if directory is a git repository
 * @param dir - Directory to check
 * @returns true if .git directory exists
 */
function isGitRepository(dir: string): boolean {
  try {
    const gitDir = join(dir, '.git');
    const stats = statSync(gitDir);
    return stats.isDirectory();
  } catch {
    return false;
  }
}

/**
 * Get list of staged files
 *
 * Staged files are files that have been added to the index (git add).
 * These are ready to be committed.
 *
 * @param dir - Repository directory
 * @param options - Configuration options (fs option is deprecated/ignored)
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
  if (options?.strict && !isGitRepository(dir)) {
    throw new Error(`Not a git repository: ${dir}`);
  }

  try {
    return napiGetStagedFiles(dir);
  } catch (error: unknown) {
    if (options?.strict) {
      throw error;
    }
    return [];
  }
}

/**
 * Get list of unstaged modified files
 *
 * Unstaged files are files that have been modified in the working directory
 * but have not been staged (git add).
 *
 * @param dir - Repository directory
 * @param options - Configuration options (fs option is deprecated/ignored)
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
  if (options?.strict && !isGitRepository(dir)) {
    throw new Error(`Not a git repository: ${dir}`);
  }

  try {
    return napiGetUnstagedFiles(dir);
  } catch (error: unknown) {
    if (options?.strict) {
      throw error;
    }
    return [];
  }
}

/**
 * Get list of untracked files
 *
 * Untracked files are files that exist in the working directory but are
 * not tracked by git (never added, not in .git/index).
 *
 * @param dir - Repository directory
 * @param options - Configuration options (fs option is deprecated/ignored)
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
  if (options?.strict && !isGitRepository(dir)) {
    throw new Error(`Not a git repository: ${dir}`);
  }

  try {
    return napiGetUntrackedFiles(dir);
  } catch (error: unknown) {
    if (options?.strict) {
      throw error;
    }
    return [];
  }
}

/**
 * Get current branch name
 *
 * @param dir - Repository directory
 * @param options - Configuration options (fs option is deprecated/ignored)
 * @returns Current branch name or undefined if not in a git repository or detached HEAD
 */
export async function getCurrentBranch(
  dir: string,
  options?: GitStatusOptions
): Promise<string | undefined> {
  if (!isGitRepository(dir)) {
    return undefined;
  }

  try {
    const branch = napiGetCurrentBranch(dir);
    return branch ?? undefined;
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

  const statusMap = new Map<string, FileStatus>();

  // Add staged files
  for (const filepath of staged) {
    statusMap.set(filepath, {
      filepath,
      staged: true,
      hasUnstagedChanges: false,
      untracked: false,
    });
  }

  // Mark files with unstaged changes
  for (const filepath of unstaged) {
    const existing = statusMap.get(filepath);
    if (existing) {
      existing.hasUnstagedChanges = true;
    } else {
      statusMap.set(filepath, {
        filepath,
        staged: false,
        hasUnstagedChanges: true,
        untracked: false,
      });
    }
  }

  // Add untracked files
  for (const filepath of untracked) {
    if (!statusMap.has(filepath)) {
      statusMap.set(filepath, {
        filepath,
        staged: false,
        hasUnstagedChanges: false,
        untracked: true,
      });
    }
  }

  return Array.from(statusMap.values());
}

/**
 * Get status for a specific file
 *
 * @param dir - Repository directory
 * @param filepath - Path to file (relative to repository root)
 * @param options - Configuration options
 * @returns FileStatus object or null if file not found
 */
export async function getFileStatus(
  dir: string,
  filepath: string,
  options?: GitStatusOptions
): Promise<FileStatus | null> {
  const allStatus = await getGitStatus(dir, options);
  return allStatus.find(s => s.filepath === filepath) ?? null;
}

/**
 * Get change type for a file using git CLI
 * @param dir - Repository directory
 * @param filepath - File path
 * @param staged - Whether to check staged or unstaged changes
 * @returns Change type: A, M, D, or R
 */
function getChangeType(
  dir: string,
  filepath: string,
  staged: boolean
): ChangeType {
  try {
    // Use git diff to get the change type
    const args = staged ? '--cached --name-status' : '--name-status';
    const output = execSync(`git diff ${args} -- "${filepath}"`, {
      cwd: dir,
      encoding: 'utf8',
      timeout: 5000,
    }).trim();

    if (!output) {
      // No diff output - check if file exists in HEAD
      try {
        execSync(`git cat-file -e HEAD:"${filepath}"`, {
          cwd: dir,
          encoding: 'utf8',
          timeout: 5000,
        });
        // File exists in HEAD but no diff - this shouldn't happen for changed files
        return 'M';
      } catch {
        // File doesn't exist in HEAD - it's an addition
        return 'A';
      }
    }

    // Parse the status letter (A, M, D, R...)
    const statusChar = output.charAt(0).toUpperCase();
    if (
      statusChar === 'A' ||
      statusChar === 'M' ||
      statusChar === 'D' ||
      statusChar === 'R'
    ) {
      return statusChar as ChangeType;
    }

    return 'M'; // Default to modified
  } catch {
    // If git command fails, check if file exists
    const fullPath = join(dir, filepath);
    if (!existsSync(fullPath)) {
      return 'D';
    }
    return 'M';
  }
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
  const files = await getStagedFiles(dir, options);

  return files.map(filepath => ({
    filepath,
    changeType: getChangeType(dir, filepath, true),
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
  const unstaged = await getUnstagedFiles(dir, options);
  const untracked = await getUntrackedFiles(dir, options);

  const result: FileStatusWithChangeType[] = [];

  // Unstaged modified files
  for (const filepath of unstaged) {
    result.push({
      filepath,
      changeType: getChangeType(dir, filepath, false),
      staged: false,
    });
  }

  // Untracked files are always 'A' (added)
  for (const filepath of untracked) {
    result.push({
      filepath,
      changeType: 'A',
      staged: false,
    });
  }

  return result;
}
