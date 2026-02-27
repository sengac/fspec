/**
 * GIT-038: Build conflict context message for the LLM.
 *
 * Pure function that constructs a user message to be sent to the Rust session
 * via sessionSendInput. This gives the LLM awareness of merge conflicts so it
 * can read the files and resolve the conflict markers.
 *
 * Separated from mergeSummaryFormatting.ts (GIT-037) for single responsibility.
 */

import { parseConflictFiles } from './mergeSummaryFormatting';

/**
 * Build a conflict resolution request message to send to the LLM.
 *
 * The returned text is sent as a user message so the LLM:
 * 1. Knows which files have conflicts
 * 2. Knows where the files are located (worktree path)
 * 3. Reads each file and resolves the git conflict markers
 * 4. Runs /merge-worktree again after resolving
 *
 * @param errorMessage - The raw Rust conflict error message
 * @param worktreePath - The absolute path to the worktree where files are located
 */
export function buildConflictLlmContext(
  errorMessage: string,
  worktreePath: string | null
): string {
  const files = parseConflictFiles(errorMessage);

  const fileList = files
    ? files.map(f => `  - ${f}`).join('\n')
    : `  (Could not parse file list from error: ${errorMessage})`;

  const locationLine = worktreePath
    ? `The files are in the worktree at: ${worktreePath}\n`
    : '';

  return (
    'Merge conflicts were detected in the following files:\n' +
    `${fileList}\n\n` +
    locationLine +
    'Please read each conflicting file, resolve the git conflict markers ' +
    '(<<<<<<< / ======= / >>>>>>>), and save the file.\n' +
    'After resolving all conflicts, run /merge-worktree again.'
  );
}
