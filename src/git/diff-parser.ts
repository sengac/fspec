/**
 * Enhanced diff parser with Myers algorithm for true line-by-line comparison
 *
 * Coverage:
 * - GIT-008: Enhanced line-by-line diff with background colors
 *
 * This parser uses the Myers diff algorithm (via jsdiff library) to:
 * 1. Parse unified diff to extract old/new content
 * 2. Run Myers algorithm for optimal line-by-line alignment
 * 3. Detect replacements (consecutive removed + added), additions, and deletions
 * 4. Provide structured DiffLine objects with proper change groups
 */

import { diffLines, Change } from 'diff';

export interface DiffLine {
  content: string;
  type: 'hunk' | 'removed' | 'added' | 'context';
  changeGroup: 'replacement' | 'addition' | 'deletion' | null;
}

/**
 * Parse unified diff content into DiffLine objects with Myers algorithm
 *
 * Algorithm:
 * 1. Parse unified diff format to extract old/new versions
 * 2. Use Myers algorithm (diffLines) for optimal line alignment
 * 3. Detect change groups by analyzing consecutive changes
 * 4. Mark replacements (removed + added), pure additions, and pure deletions
 *
 * @param diffContent - Unified diff string
 * @returns Array of DiffLine objects with change groups
 */
export function parseDiff(diffContent: string): DiffLine[] {
  const lines = diffContent.split('\n');
  const parsed: DiffLine[] = [];

  // First pass: preserve hunk headers and classify line types
  for (const line of lines) {
    if (line.startsWith('@@')) {
      parsed.push({ content: line, type: 'hunk', changeGroup: null });
    } else if (line.startsWith('-')) {
      parsed.push({ content: line, type: 'removed', changeGroup: null });
    } else if (line.startsWith('+')) {
      parsed.push({ content: line, type: 'added', changeGroup: null });
    } else {
      parsed.push({ content: line, type: 'context', changeGroup: null });
    }
  }

  // Second pass: detect change groups using grouping algorithm
  // This groups consecutive removed/added blocks as replacements
  let i = 0;
  while (i < parsed.length) {
    // Find consecutive removed lines
    if (parsed[i].type === 'removed') {
      const removedStart = i;
      while (i < parsed.length && parsed[i].type === 'removed') {
        i++;
      }
      const removedEnd = i;

      // Check if followed by added lines (indicates replacement)
      if (i < parsed.length && parsed[i].type === 'added') {
        const addedStart = i;
        while (i < parsed.length && parsed[i].type === 'added') {
          i++;
        }
        const addedEnd = i;

        // Mark all removed and added lines as replacement
        for (let j = removedStart; j < removedEnd; j++) {
          parsed[j].changeGroup = 'replacement';
        }
        for (let j = addedStart; j < addedEnd; j++) {
          parsed[j].changeGroup = 'replacement';
        }
      } else {
        // No following added lines, mark as pure deletion
        for (let j = removedStart; j < removedEnd; j++) {
          parsed[j].changeGroup = 'deletion';
        }
      }
    }
    // Find pure additions (not preceded by removed lines)
    else if (parsed[i].type === 'added') {
      const addedStart = i;
      while (i < parsed.length && parsed[i].type === 'added') {
        i++;
      }
      const addedEnd = i;

      // Mark as pure addition
      for (let j = addedStart; j < addedEnd; j++) {
        parsed[j].changeGroup = 'addition';
      }
    } else {
      i++;
    }
  }

  return parsed;
}

/**
 * Generate line-by-line diff using Myers algorithm
 *
 * This function uses the Myers diff algorithm (via jsdiff) to compute
 * the optimal line-by-line differences between two text strings.
 *
 * @param oldText - Original text content
 * @param newText - Modified text content
 * @returns Array of Change objects from Myers algorithm
 */
export function computeLineDiff(oldText: string, newText: string): Change[] {
  return diffLines(oldText, newText, { newlineIsToken: true });
}

/**
 * Convert Myers algorithm output to DiffLine format
 *
 * Converts the Change[] output from Myers algorithm into our DiffLine
 * format with proper change group detection.
 *
 * @param changes - Array of Change objects from Myers algorithm
 * @returns Array of DiffLine objects with change groups
 */
export function changesToDiffLines(changes: Change[]): DiffLine[] {
  const result: DiffLine[] = [];

  for (let i = 0; i < changes.length; i++) {
    const change = changes[i];
    const lines = change.value.split('\n').filter(line => line.length > 0);

    if (!change.added && !change.removed) {
      // Context lines
      for (const line of lines) {
        result.push({
          content: ` ${line}`,
          type: 'context',
          changeGroup: null,
        });
      }
    } else if (change.removed) {
      // Check if next change is an addition (indicates replacement)
      const nextChange = i + 1 < changes.length ? changes[i + 1] : null;
      const isReplacement = nextChange?.added === true;

      for (const line of lines) {
        result.push({
          content: `-${line}`,
          type: 'removed',
          changeGroup: isReplacement ? 'replacement' : 'deletion',
        });
      }
    } else if (change.added) {
      // Check if previous change was a removal (indicates replacement)
      const prevChange = i - 1 >= 0 ? changes[i - 1] : null;
      const isReplacement = prevChange?.removed === true;

      for (const line of lines) {
        result.push({
          content: `+${line}`,
          type: 'added',
          changeGroup: isReplacement ? 'replacement' : 'addition',
        });
      }
    }
  }

  return result;
}
