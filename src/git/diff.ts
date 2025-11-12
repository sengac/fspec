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
    let headBuffer = Buffer.alloc(0);
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
      headBuffer = Buffer.from(blob);
      headContent = headBuffer.toString('utf8');
    } catch {
      // File doesn't exist in HEAD (new file)
      headContent = '';
    }

    // Get working directory version
    const workingBuffer = await fs.readFile(fullPath);
    const workingContent = workingBuffer.toString('utf8');

    // Check if either version is binary
    if (isBinaryContent(headBuffer) || isBinaryContent(workingBuffer)) {
      return '[Binary file - no diff available]';
    }

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
 * Get diff between a checkpoint file and HEAD
 * @param cwd - Working directory path
 * @param filepath - Relative path to file from cwd
 * @param checkpointRef - Git ref for the checkpoint
 * @returns Unified diff string or null if no changes
 */
export async function getCheckpointFileDiff(
  cwd: string,
  filepath: string,
  checkpointRef: string
): Promise<string | null> {
  try {
    // Get checkpoint version of file
    let checkpointBuffer = Buffer.alloc(0);
    let checkpointContent = '';
    try {
      const checkpointOid = await git.resolveRef({
        fs: fsNode,
        dir: cwd,
        ref: checkpointRef,
      });
      const { blob } = await git.readBlob({
        fs: fsNode,
        dir: cwd,
        oid: checkpointOid,
        filepath,
      });
      checkpointBuffer = Buffer.from(blob);
      checkpointContent = checkpointBuffer.toString('utf8');
    } catch {
      // File doesn't exist in checkpoint - will be deleted on restore
      return `Will be deleted on restore: ${filepath}\n\nThis file exists in HEAD but not in the checkpoint.\nRestoring the checkpoint will remove this file from the working directory.`;
    }

    // Get HEAD version of file (if it exists)
    let headBuffer = Buffer.alloc(0);
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
      headBuffer = Buffer.from(blob);
      headContent = headBuffer.toString('utf8');
    } catch {
      // File doesn't exist in HEAD (was deleted)
      headContent = '';
    }

    // Check if either version is binary
    if (isBinaryContent(headBuffer) || isBinaryContent(checkpointBuffer)) {
      return '[Binary file - no diff available]';
    }

    // If contents are identical, no diff
    if (checkpointContent === headContent) {
      return 'No changes between checkpoint and HEAD';
    }

    // Generate unified diff (HEAD as "old", checkpoint as "new") to show restore preview
    return generateUnifiedDiff(filepath, headContent, checkpointContent);
  } catch (error) {
    throw new Error(
      `Failed to get checkpoint diff for ${filepath}: ${error instanceof Error ? error.message : String(error)}`
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

  // Use Myers algorithm (via diff library) for proper line alignment
  const changes: Change[] = diffLines(oldContent, newContent, {
    newlineIsToken: false,
  });

  // Count added and removed lines for the header
  let addedCount = 0;
  let removedCount = 0;
  const chunks: string[] = [];

  for (const change of changes) {
    const lines = change.value.split('\n').filter(line => line.length > 0);

    if (!change.added && !change.removed) {
      // Context lines (unchanged)
      for (const line of lines) {
        chunks.push(` ${line}`);
      }
    } else if (change.removed) {
      // Removed lines (present in checkpoint but not HEAD)
      removedCount += lines.length;
      for (const line of lines) {
        chunks.push(`-${line}`);
      }
    } else if (change.added) {
      // Added lines (present in HEAD but not checkpoint)
      addedCount += lines.length;
      for (const line of lines) {
        chunks.push(`+${line}`);
      }
    }
  }

  // Build diff with user-friendly headers
  const diff: string[] = [];
  diff.push(`--- Lines that will be REMOVED on restore: ${removedCount} lines`);
  diff.push(`+++ Lines that will be ADDED on restore: ${addedCount} lines`);

  // Truncate if more than 20,000 lines
  const MAX_LINES = 20000;
  const totalLines = Math.max(oldLines.length, newLines.length);

  if (chunks.length > MAX_LINES) {
    const truncatedChunks = chunks.slice(0, MAX_LINES);
    diff.push(...truncatedChunks);
    diff.push('');
    diff.push(
      `[File truncated - showing first ${MAX_LINES.toLocaleString()} of ${totalLines.toLocaleString()} lines]`
    );
  } else {
    diff.push(...chunks);
  }

  return diff.join('\n');
}

/**
 * Check if buffer content contains binary data
 * @param buffer - Buffer to check
 * @returns true if binary, false if text
 */
function isBinaryContent(buffer: Buffer): boolean {
  if (buffer.length === 0) {
    return false;
  }

  // Check for null bytes (common indicator of binary content)
  // Scan first 8000 bytes for performance
  const checkLength = Math.min(buffer.length, 8000);
  for (let i = 0; i < checkLength; i++) {
    if (buffer[i] === 0) {
      return true;
    }
  }

  return false;
}
