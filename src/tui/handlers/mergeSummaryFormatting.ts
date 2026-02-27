/**
 * Merge summary formatting utilities.
 *
 * Pure functions for building rich text summaries from merge results
 * and conflict error messages. Extracted from mergeWorktreeHandler.ts
 * for separation of concerns and testability.
 *
 * GIT-037 (buildConflictLlmContext moved to conflictLlmContext.ts for GIT-038)
 */

/**
 * Build a rich file-by-file merge summary.
 *
 * Format:
 *   ✓ Merge successful
 *
 *     Modified (3):
 *       src/auth/login.ts
 *       ...
 *
 *     Added (1):
 *       src/auth/types.ts
 *
 *     Deleted (0)
 */
export function buildMergeSummary(mergeResult: {
  filesModified: string[];
  filesAdded: string[];
  filesDeleted: string[];
}): string {
  const lines: string[] = ['✓ Merge successful', ''];

  const { filesModified, filesAdded, filesDeleted } = mergeResult;

  // Modified
  lines.push(`  Modified (${filesModified.length}):`);
  if (filesModified.length > 0) {
    for (const f of filesModified) {
      lines.push(`    ${f}`);
    }
  }
  lines.push('');

  // Added
  lines.push(`  Added (${filesAdded.length}):`);
  if (filesAdded.length > 0) {
    for (const f of filesAdded) {
      lines.push(`    ${f}`);
    }
  }
  lines.push('');

  // Deleted
  lines.push(`  Deleted (${filesDeleted.length})`);
  if (filesDeleted.length > 0) {
    for (const f of filesDeleted) {
      lines.push(`    ${f}`);
    }
  }

  return lines.join('\n');
}

/**
 * Parse conflict file paths from Rust error message.
 *
 * Rust format (Debug trait on Vec<String>):
 *   'Conflict detected: ["file1.ts", "file2.ts"] have been modified in both session and main worktree'
 *
 * Falls back to null if parsing fails.
 */
export function parseConflictFiles(errorMessage: string): string[] | null {
  const match = errorMessage.match(/\[([^\]]+)\]/);
  if (!match) {
    return null;
  }

  try {
    // Extract paths from the bracket content: "file1.ts", "file2.ts"
    const paths = match[1].split(',').map(s => s.trim().replace(/^"|"$/g, ''));
    if (paths.length > 0 && paths[0].length > 0) {
      return paths;
    }
  } catch {
    // Fall through to null
  }

  return null;
}

/**
 * Build a rich conflict summary.
 */
export function buildConflictSummary(errorMessage: string): string {
  const files = parseConflictFiles(errorMessage);

  if (!files) {
    // Graceful degradation: show raw error with guidance
    return `⚠ Merge conflicts detected\n\n  ${errorMessage}\n\n  Resolve the conflicts, then run /merge-worktree again.`;
  }

  const lines: string[] = ['⚠ Merge conflicts detected', ''];
  lines.push('  Conflicting files:');
  for (const f of files) {
    lines.push(`    ${f}`);
  }
  lines.push('');
  lines.push(
    '  These files were modified in both this session and the main worktree.'
  );
  lines.push('  Resolve the conflicts, then run /merge-worktree again.');

  return lines.join('\n');
}
