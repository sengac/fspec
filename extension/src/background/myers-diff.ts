/**
 * fspec Browser Agent - Myers Diff Algorithm
 *
 * Simplified line-level text diff using the Myers algorithm.
 * Produces the shortest edit script between two arrays of strings.
 * Used by browser_diff_page to diff accessibility tree text.
 *
 * Pure function — no Chrome dependencies, fully testable in Node/jsdom.
 *
 * Implemented by: LOCATE-006
 */

/** Individual diff operation */
export interface DiffLine {
  type: 'add' | 'remove' | 'equal';
  text: string;
}

/** Diff statistics */
export interface DiffStats {
  additions: number;
  removals: number;
  unchanged: number;
  changed: boolean;
}

/** Complete diff result */
export interface DiffResult {
  lines: DiffLine[];
  stats: DiffStats;
}

/**
 * Compute the shortest edit script between two line arrays
 * using the Myers diff algorithm.
 */
export function myersDiff(oldLines: string[], newLines: string[]): DiffResult {
  const n = oldLines.length;
  const m = newLines.length;
  const max = n + m;

  if (max === 0) {
    return {
      lines: [],
      stats: { additions: 0, removals: 0, unchanged: 0, changed: false },
    };
  }

  // V array indexed by diagonal k, storing the furthest-reaching x on each diagonal
  // Using Map to support negative indices
  const trace: Map<number, number>[] = [];

  let found = false;
  outer: for (let d = 0; d <= max; d++) {
    const v = new Map<number, number>();
    trace.push(v);

    for (let k = -d; k <= d; k += 2) {
      let x: number;
      if (
        k === -d ||
        (k !== d &&
          (getPrev(trace, d - 1, k - 1) ?? 0) <
            (getPrev(trace, d - 1, k + 1) ?? 0))
      ) {
        x = getPrev(trace, d - 1, k + 1) ?? 0; // Move down (insert)
      } else {
        x = (getPrev(trace, d - 1, k - 1) ?? 0) + 1; // Move right (delete)
      }
      let y = x - k;

      // Follow diagonal (equal lines)
      while (x < n && y < m && oldLines[x] === newLines[y]) {
        x++;
        y++;
      }

      v.set(k, x);

      if (x >= n && y >= m) {
        found = true;
        break outer;
      }
    }
  }

  if (!found && max > 0) {
    // Fallback: shouldn't happen with correct Myers, but safety net
    return buildFallback(oldLines, newLines);
  }

  // Backtrack to recover the edit script
  const edits: DiffLine[] = [];
  let x = n;
  let y = m;

  for (let d = trace.length - 1; d > 0; d--) {
    const k = x - y;
    const prevV = trace[d - 1];

    let prevK: number;
    if (
      k === -d ||
      (k !== d && (prevV.get(k - 1) ?? 0) < (prevV.get(k + 1) ?? 0))
    ) {
      prevK = k + 1; // Was a down move (insert)
    } else {
      prevK = k - 1; // Was a right move (delete)
    }

    const prevX = prevV.get(prevK) ?? 0;
    const prevY = prevX - prevK;

    // Diagonal moves (equal lines) from (prevX, prevY) to the pre-move position
    let cx = x;
    let cy = y;
    while (
      cx > prevX + (prevK < k ? 1 : 0) &&
      cy > prevY + (prevK > k ? 1 : 0)
    ) {
      cx--;
      cy--;
      edits.push({ type: 'equal', text: oldLines[cx] });
    }

    if (prevK < k) {
      // Right move: delete from old
      edits.push({ type: 'remove', text: oldLines[prevX] });
    } else if (prevK > k) {
      // Down move: insert from new
      edits.push({ type: 'add', text: newLines[prevY] });
    }

    x = prevX;
    y = prevY;
  }

  // Handle remaining diagonal at d=0
  while (x > 0 && y > 0) {
    x--;
    y--;
    edits.push({ type: 'equal', text: oldLines[x] });
  }

  edits.reverse();

  // Compute stats
  let additions = 0;
  let removals = 0;
  let unchanged = 0;
  for (const edit of edits) {
    if (edit.type === 'add') {
      additions++;
    } else if (edit.type === 'remove') {
      removals++;
    } else {
      unchanged++;
    }
  }

  return {
    lines: edits,
    stats: {
      additions,
      removals,
      unchanged,
      changed: additions > 0 || removals > 0,
    },
  };
}

function getPrev(
  trace: Map<number, number>[],
  d: number,
  k: number
): number | undefined {
  if (d < 0 || d >= trace.length) {
    return undefined;
  }
  return trace[d].get(k);
}

function buildFallback(oldLines: string[], newLines: string[]): DiffResult {
  const lines: DiffLine[] = [];
  for (const line of oldLines) {
    lines.push({ type: 'remove', text: line });
  }
  for (const line of newLines) {
    lines.push({ type: 'add', text: line });
  }
  return {
    lines,
    stats: {
      additions: newLines.length,
      removals: oldLines.length,
      unchanged: 0,
      changed: oldLines.length > 0 || newLines.length > 0,
    },
  };
}

/**
 * Format a DiffResult into a human-readable unified diff string
 * with context lines around changes and ellipsis for gaps.
 */
export function formatDiffOutput(result: DiffResult): string {
  const { lines, stats } = result;
  const CONTEXT = 1; // lines of context around changes

  if (!stats.changed) {
    const summary =
      `${stats.additions} addition${stats.additions !== 1 ? 's' : ''}, ` +
      `${stats.removals} removal${stats.removals !== 1 ? 's' : ''}, ` +
      `${stats.unchanged} unchanged`;
    return `No changes detected.\n\nChanges: ${summary}`;
  }

  // Find which lines are near changes
  const isChange = lines.map(l => l.type !== 'equal');
  const include = new Array<boolean>(lines.length).fill(false);

  for (let i = 0; i < lines.length; i++) {
    if (isChange[i]) {
      for (
        let j = Math.max(0, i - CONTEXT);
        j <= Math.min(lines.length - 1, i + CONTEXT);
        j++
      ) {
        include[j] = true;
      }
    }
  }

  // Build output with ellipsis for gaps
  const outputLines: string[] = [];
  let lastIncluded = -1;

  for (let i = 0; i < lines.length; i++) {
    if (!include[i]) {
      continue;
    }
    // Add ellipsis when there's a gap (excluded lines before this included line)
    if (
      (lastIncluded === -1 && i > 0) ||
      (lastIncluded >= 0 && i - lastIncluded > 1)
    ) {
      outputLines.push('...');
    }
    const line = lines[i];
    if (line.type === 'add') {
      outputLines.push(`+ ${line.text}`);
    } else if (line.type === 'remove') {
      outputLines.push(`- ${line.text}`);
    } else {
      outputLines.push(`  ${line.text}`);
    }
    lastIncluded = i;
  }

  // Add trailing ellipsis if there are omitted lines after the last included line
  if (lastIncluded >= 0 && lastIncluded < lines.length - 1) {
    outputLines.push('...');
  }

  const summary =
    `${stats.additions} addition${stats.additions !== 1 ? 's' : ''}, ` +
    `${stats.removals} removal${stats.removals !== 1 ? 's' : ''}, ` +
    `${stats.unchanged} unchanged`;

  return `${outputLines.join('\n')}\n\nChanges: ${summary}`;
}
