/**
 * Git context detection for virtual hooks
 */

import { execa } from 'execa';

export interface GitContext {
  stagedFiles: string[];
  unstagedFiles: string[];
}

/**
 * Detect git context (staged and unstaged files)
 * @param projectRoot - Project root directory
 * @returns Git context with file lists
 */
export async function getGitContext(
  projectRoot: string
): Promise<GitContext> {
  try {
    // Detect staged files (git diff --cached --name-only)
    const stagedResult = await execa('git', ['diff', '--cached', '--name-only'], {
      cwd: projectRoot,
      reject: false,
    });

    const stagedFiles =
      stagedResult.exitCode === 0 && stagedResult.stdout.trim()
        ? stagedResult.stdout.trim().split('\n')
        : [];

    // Detect unstaged files (git diff --name-only)
    const unstagedResult = await execa('git', ['diff', '--name-only'], {
      cwd: projectRoot,
      reject: false,
    });

    const unstagedFiles =
      unstagedResult.exitCode === 0 && unstagedResult.stdout.trim()
        ? unstagedResult.stdout.trim().split('\n')
        : [];

    return {
      stagedFiles,
      unstagedFiles,
    };
  } catch (error: unknown) {
    // If git is not available or not a git repo, return empty arrays
    return {
      stagedFiles: [],
      unstagedFiles: [],
    };
  }
}
