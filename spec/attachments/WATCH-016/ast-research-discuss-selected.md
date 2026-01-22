# AST Research: Discuss Selected Feature (WATCH-016)

## Overview

Research to understand the existing implementation and determine what changes are needed for the "Discuss Selected" feature in the watcher split view.

## Key Findings

### 1. SplitSessionView Component
**Location:** `src/tui/components/SplitSessionView.tsx:91`

```
fspec research --tool=ast --pattern='export const SplitSessionView' --lang=tsx --path=src/tui/components/SplitSessionView.tsx
→ src/tui/components/SplitSessionView.tsx:91:1
```

The split view component already exists and handles the dual-pane display.

### 2. Existing Enter Key Handler for Parent Pane
**Location:** `src/tui/components/SplitSessionView.tsx:208`

```
fspec research --tool=ast --pattern='key.return' --lang=tsx --path=src/tui/components/SplitSessionView.tsx
→ src/tui/components/SplitSessionView.tsx:208:9
```

The Enter key handler for the parent pane **already exists** at line 208. It:
- Checks `key.return && activePane === 'parent' && parentSelection.isSelectMode`
- Calls `generateDiscussSelectedPrefill()` to create the pre-fill text
- Calls `onInputChange(prefill)` to update the input
- Calls `parentSelection.exitSelectMode()` to exit select mode

### 3. generateDiscussSelectedPrefill Function
**Location:** `src/tui/utils/turnSelection.ts:112`

```
fspec research --tool=ast --pattern='function generateDiscussSelectedPrefill' --lang=typescript --path=src/tui/utils/turnSelection.ts
→ src/tui/utils/turnSelection.ts:112:8
```

The utility function exists and:
- Takes turnNumber (1-indexed) and turnContent
- Truncates to 50 characters with "..."
- Returns formatted string: `Regarding turn N in parent session:\n\`\`\`\npreview\n\`\`\`\n`

### 4. TurnContentModal Component
**Location:** `src/tui/components/TurnContentModal.tsx:119`

```
fspec research --tool=ast --pattern='export const TurnContentModal' --lang=tsx --path=src/tui/components/TurnContentModal.tsx
→ src/tui/components/TurnContentModal.tsx:119:1
```

The TurnContentModal component exists in AgentView and is used for viewing full turn content.

## Implementation Status

### Parent Pane Enter Behavior: ✅ ALREADY IMPLEMENTED
The "Discuss Selected" feature for the parent pane is **already implemented** in SplitSessionView.tsx:
- Lines 207-224 handle Enter key in parent pane select mode
- Uses `generateDiscussSelectedPrefill()` from turnSelection.ts
- Pre-fills input with formatted context
- Exits select mode after pre-fill

### Watcher Pane Enter Behavior: ❌ NOT IMPLEMENTED
Enter key handling for the watcher pane to open TurnContentModal is **NOT implemented** in SplitSessionView.tsx. The current code only handles:
1. Parent pane Enter → pre-fill input (lines 207-224)

Missing: Watcher pane Enter → open TurnContentModal

## Implementation Required

Add Enter key handler for watcher pane in SplitSessionView.tsx:
```typescript
// After the parent pane Enter handler (line 224)
// Enter in watcher pane select mode: Open TurnContentModal
if (key.return && activePane === 'watcher' && watcherSelection.isSelectMode) {
  const selectedIndex = watcherSelection.selectionRef.current.selectedIndex;
  const selectedLine = watcherConversation[selectedIndex];
  
  if (selectedLine) {
    // Need to call a callback to open TurnContentModal
    // This requires adding an onOpenTurnContent prop to SplitSessionView
    onOpenTurnContent?.(selectedLine.messageIndex);
  }
  return;
}
```

However, this requires:
1. Adding `onOpenTurnContent` prop to SplitSessionViewProps
2. Passing the callback from AgentView
3. AgentView setting `showTurnContent` and `selectedTurnContent` state

## Conclusion

The parent pane "Discuss Selected" feature is already complete. Only the watcher pane Enter behavior (opening TurnContentModal) needs implementation.
