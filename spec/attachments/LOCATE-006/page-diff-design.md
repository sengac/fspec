# LOCATE-006: Page Diff Tool — Design Document

## Purpose

The `browser_diff_page` tool enables the **verify** step in the scan→interact→verify workflow. After an AI agent performs actions (click, fill), it calls `browser_diff_page` to see what changed on the page, confirming the action had the intended effect.

## Algorithm: Simplified Myers Diff

We implement a line-level text diff using the Myers algorithm. The input is two arrays of strings (lines from the tree text), and the output is a unified diff with `+` additions and `-` removals.

### Why Myers (not simple line comparison)

- Myers finds the **minimum edit distance** — produces the most readable diffs
- Line-level diff is sufficient (we're comparing tree notation, not raw HTML)
- The implementation is ~80 lines of TypeScript
- agent-browser uses the same approach (src/diff.ts)

### Algorithm Pseudocode

```
function myersDiff(oldLines: string[], newLines: string[]): DiffResult {
  // Myers shortest edit script
  const n = oldLines.length;
  const m = newLines.length;
  const max = n + m;
  const v = new Map<number, number>();
  const trace: Map<number, number>[] = [];
  
  // Forward pass: find shortest edit script
  v.set(1, 0);
  for (d = 0..max) {
    for (k = -d..d step 2) {
      // Choose between going down (insert) or right (delete)
      // Follow diagonal (equal lines)
      // Record trace for backtracking
    }
    if (reached end) break;
  }
  
  // Backtrack to get edit operations
  // Generate unified diff output
}
```

## Handler Implementation

```typescript
handlers.set('browser_diff_page', async (args) => {
  const tabId = await resolveTabId(args.tabId);
  
  // 1. Get previous scan state
  const previousState = getTabScanState(tabId);
  const previousTreeText = previousState?.treeText ?? '';
  
  // 2. Run fresh scan (same function as browser_scan_page)
  const results = await scripting.executeScript({
    target: { tabId },
    func: scanPageDOM,  // Shared with browser_scan_page
  });
  
  // 3. Process scan results (assign refs, format tree)
  const newTreeText = formatAccessibilityTree(results);
  
  // 4. Update stored state
  setTabScanState(tabId, { refs: newRefs, treeText: newTreeText, timestamp: Date.now() });
  
  // 5. Compute diff
  const oldLines = previousTreeText.split('\n');
  const newLines = newTreeText.split('\n');
  const diff = myersDiff(oldLines, newLines);
  
  // 6. Format output
  const output = formatDiffOutput(diff);
  return textResult(output);
});
```

## Output Format

```
  - heading "Sign In" [level=1]
  - textbox "Email" [ref=e1] [type=email]
  - textbox "Password" [ref=e2] [type=password]
- - button "Sign In" [ref=e3]
+ - button "Signing in..." [ref=e3] [disabled]

Changes: 1 addition, 1 removal, 3 unchanged
```

### First Scan (No Previous State)

If `browser_diff_page` is called without a previous `browser_scan_page`:
- Treat all current lines as additions (`+ ...`)
- Return with note: "No previous scan to compare against. Showing current state."

## Shared Scanning Function

The scanning function should be extractable so both `browser_scan_page` and `browser_diff_page` can use it. Options:

1. **Extract to separate module** (e.g., `dom-scanner.ts`) — cleanest
2. **Define once in browser-tools.ts scope** — simpler, still shared via closure

Recommendation: Option 2 for V1, refactor to Option 1 if the file exceeds 300 lines.

## Diff Stats

```typescript
interface DiffStats {
  additions: number;
  removals: number;
  unchanged: number;
  changed: boolean;
}
```

The `changed` boolean is a convenience for the AI: if `!changed`, the action had no visible effect.

## Token Efficiency

Only include changed regions with 1-2 lines of context (unchanged lines around changes). This keeps the diff output small even for large pages.

## Tool Input Schema

```json
{
  "type": "object",
  "properties": {
    "tabId": {
      "type": "number",
      "description": "Tab to diff (defaults to active tab)"
    }
  }
}
```

## Testing Strategy

1. **Identical pages**: Diff returns 0 changes
2. **Single line change**: One addition + one removal
3. **New elements added**: Multiple additions
4. **Elements removed**: Multiple removals
5. **Complete page change** (navigation): All additions
6. **No previous scan**: Graceful handling, returns current state
7. **Empty page**: Handles edge case
