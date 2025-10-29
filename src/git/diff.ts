/**
 * Git diff operations using isomorphic-git
 */

import git from 'isomorphic-git';
import fsNode from 'fs';
import path from 'path';
import { diffLines, Change } from 'diff';

/**
 * Get unified diff for a specific file
 * @param cwd - Working directory path
 * @param filepath - Relative path to file from cwd
 * @returns Unified diff string or null if no changes
 */
export async function getFileDiff(
  cwd: string,
  filepath: string
): Promise<string | null> {
  try {
    const fullPath = path.join(cwd, filepath);
    const fs = fsNode.promises;

    // Check if file exists
    try {
      await fs.access(fullPath);
    } catch {
      return null;
    }

    // Get HEAD version of file (if it exists)
    let headContent = '';
    try {
      const headCommitOid = await git.resolveRef({
        fs: fsNode,
        dir: cwd,
        ref: 'HEAD',
      });
      const { blob } = await git.readBlob({
        fs: fsNode,
        dir: cwd,
        oid: headCommitOid,
        filepath,
      });
      headContent = Buffer.from(blob).toString('utf8');
    } catch (error) {
      // File doesn't exist in HEAD (new file)
      headContent = '';
    }

    // Get working directory version
    const workingContent = await fs.readFile(fullPath, 'utf8');

    // If contents are identical, no diff
    if (headContent === workingContent) {
      return null;
    }

    // Generate unified diff
    return generateUnifiedDiff(filepath, headContent, workingContent);
  } catch (error) {
    throw new Error(
      `Failed to get diff for ${filepath}: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Generate unified diff format from two strings using Myers algorithm
 * @param filepath - File path for diff header
 * @param oldContent - Original content
 * @param newContent - New content
 * @returns Unified diff string
 */
function generateUnifiedDiff(
  filepath: string,
  oldContent: string,
  newContent: string
): string {
  const oldLines = oldContent.split('\n');
  const newLines = newContent.split('\n');

  const diff: string[] = [];
  diff.push(`diff --git a/${filepath} b/${filepath}`);
  diff.push(`--- a/${filepath}`);
  diff.push(`+++ b/${filepath}`);

  // Use Myers algorithm (via diff library) for proper line alignment
  const changes: Change[] = diffLines(oldContent, newContent, {
    newlineIsToken: false,
  });

  const chunks: string[] = [];
  for (const change of changes) {
    const lines = change.value
      .split('\n')
      .filter(line => line.length > 0 || change.value.endsWith('\n'));

    if (!change.added && !change.removed) {
      // Context lines (unchanged)
      for (const line of lines) {
        chunks.push(` ${line}`);
      }
    } else if (change.removed) {
      // Removed lines
      for (const line of lines) {
        chunks.push(`-${line}`);
      }
    } else if (change.added) {
      // Added lines
      for (const line of lines) {
        chunks.push(`+${line}`);
      }
    }
  }

  // Add hunk header
  diff.push(`@@ -1,${oldLines.length} +1,${newLines.length} @@`);
  diff.push(...chunks);

  return diff.join('\n');
}
