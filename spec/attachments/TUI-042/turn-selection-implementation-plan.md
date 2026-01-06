# TUI-042: Turn Selection Implementation Plan

## Overview

This document outlines the implementation strategy for changing the `/select` command from line-based selection to turn-based (message-based) selection in AgentView.

## Current State Analysis

### Data Structures

```typescript
// Current ConversationLine already has messageIndex!
interface ConversationLine {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  messageIndex: number;  // ← This IS the turn index
}

// Lines are generated from conversation messages
const conversationLines = useMemo((): ConversationLine[] => {
  deferredConversation.forEach((msg, msgIndex) => {
    const messageLines = wrapMessageToLines(msg, msgIndex, maxWidth);
    lines.push(...messageLines);
  });
  return lines;
}, [deferredConversation, terminalWidth]);
```

**Key insight**: Each `ConversationLine` already tracks its parent message via `messageIndex`. This is the turn index we need for turn-based selection.

### Current TUI-041 Implementation

```typescript
// State (line 594)
const [isLineSelectMode, setIsLineSelectMode] = useState(false);

// VirtualList usage (line 4850)
selectionMode={isLineSelectMode ? 'item' : 'scroll'}

// renderItem checks isSelected for individual line highlighting
renderItem={(line, _index, isSelected) => {
  const color = isSelected ? 'cyan' : baseColor;
  // ... cyan highlight on single line
}}
```

The current implementation:
1. Toggles between VirtualList's `'item'` (line selection) and `'scroll'` (no selection) modes
2. VirtualList handles navigation between individual lines internally
3. `isSelected` is `true` only for the single selected line

## Proposed Changes

### High-Level Approach

**Replace line selection with turn selection:**
1. Rename `isLineSelectMode` → `isTurnSelectMode` (or keep same name, change semantics)
2. Track `selectedTurnIndex` explicitly in AgentView state
3. Use VirtualList in `selectionMode='scroll'` but handle navigation at AgentView level
4. In `renderItem`, highlight ALL lines where `line.messageIndex === selectedTurnIndex`
5. Intercept arrow keys to navigate between turns (not lines)

### Detailed Implementation Plan

#### Step 1: State Changes

```typescript
// Replace line-based state with turn-based state
const [isTurnSelectMode, setIsTurnSelectMode] = useState(false);
const [selectedTurnIndex, setSelectedTurnIndex] = useState<number | null>(null);

// Track turn boundaries for efficient navigation
const turnBoundaries = useMemo(() => {
  // Returns array of { turnIndex, firstLineIndex, lastLineIndex }
  const boundaries: Array<{ turnIndex: number; firstLine: number; lastLine: number }> = [];
  let currentTurn = -1;
  let firstLine = 0;
  
  conversationLines.forEach((line, idx) => {
    if (line.messageIndex !== currentTurn) {
      if (currentTurn >= 0) {
        boundaries.push({ turnIndex: currentTurn, firstLine, lastLine: idx - 1 });
      }
      currentTurn = line.messageIndex;
      firstLine = idx;
    }
  });
  
  // Don't forget the last turn
  if (currentTurn >= 0) {
    boundaries.push({ turnIndex: currentTurn, firstLine, lastLine: conversationLines.length - 1 });
  }
  
  return boundaries;
}, [conversationLines]);
```

#### Step 2: VirtualList Configuration

When in turn selection mode:
```typescript
<VirtualList
  items={conversationLines}
  selectionMode="scroll"  // Always scroll mode - we handle selection
  isFocused={!isTurnSelectMode && !showProviderSelector && ...}  // Disable VL input in turn mode
  scrollToEnd={true}
  // ... other props
/>
```

**Key**: Set `isFocused={false}` when in turn selection mode to prevent VirtualList from handling arrow keys. Handle input at AgentView level instead.

#### Step 3: Turn Navigation (useInput at AgentView level)

```typescript
// Handle turn navigation when in turn selection mode
useInput(
  (input, key) => {
    if (!isTurnSelectMode) return;
    
    if (key.upArrow) {
      // Navigate to previous turn
      setSelectedTurnIndex(prev => {
        if (prev === null) return turnBoundaries.length - 1;
        return Math.max(0, prev - 1);
      });
    } else if (key.downArrow) {
      // Navigate to next turn
      setSelectedTurnIndex(prev => {
        if (prev === null) return 0;
        return Math.min(turnBoundaries.length - 1, prev + 1);
      });
    }
  },
  { isActive: isTurnSelectMode }
);
```

#### Step 4: Scroll to Selected Turn

```typescript
// When selectedTurnIndex changes, scroll to show the turn
useEffect(() => {
  if (!isTurnSelectMode || selectedTurnIndex === null) return;
  
  const boundary = turnBoundaries.find(b => b.turnIndex === selectedTurnIndex);
  if (boundary) {
    // Scroll to show the first line of the selected turn
    // Need to expose scrollTo from VirtualList via ref or callback
  }
}, [selectedTurnIndex, isTurnSelectMode, turnBoundaries]);
```

**Challenge**: VirtualList doesn't currently expose an imperative `scrollTo` API. Options:
1. Add `scrollToIndex` prop to VirtualList
2. Use `forwardRef` and `useImperativeHandle` in VirtualList
3. Manage scroll state at AgentView level and pass to VirtualList

**Recommended**: Add `initialScrollIndex` or `scrollToIndex` prop to VirtualList that triggers scroll when changed.

#### Step 5: Render Item with Turn Highlighting

```typescript
renderItem={(line, _index, _isSelected) => {
  // In turn selection mode, check if this line's turn is selected
  const isTurnSelected = isTurnSelectMode && line.messageIndex === selectedTurnIndex;
  
  // Apply highlighting to ALL lines in the selected turn
  const selectionPrefix = isTurnSelected ? '> ' : '  ';
  const baseColor = line.role === 'user' ? 'green' : 'white';
  const color = isTurnSelected ? 'cyan' : baseColor;
  
  return (
    <Box flexGrow={1}>
      {isTurnSelected && <Text color="cyan">{selectionPrefix}</Text>}
      <Text color={color}>{content}</Text>
    </Box>
  );
}}
```

#### Step 6: Update /select Command Handler

```typescript
if (userMessage === '/select') {
  setInputValue('');
  const newMode = !isTurnSelectMode;
  setIsTurnSelectMode(newMode);
  
  if (newMode) {
    // Initialize to last turn when enabling
    setSelectedTurnIndex(deferredConversation.length - 1);
  } else {
    setSelectedTurnIndex(null);
  }
  
  setConversation(prev => [
    ...prev,
    {
      role: 'tool',
      content: `Turn selection mode ${newMode ? 'enabled' : 'disabled'}. ${
        newMode
          ? 'Arrow keys now navigate between conversation turns.'
          : 'Arrow keys now scroll the viewport.'
      }`,
    },
  ]);
  return;
}
```

## VirtualList Enhancements Required

### Option A: Add `scrollToIndex` prop (Recommended)

```typescript
interface VirtualListProps<T> {
  // ... existing props
  scrollToIndex?: number;  // When set, scroll to show this index
}

// In VirtualList component:
useEffect(() => {
  if (scrollToIndex !== undefined) {
    const offset = Math.max(0, Math.min(scrollToIndex, maxScrollOffset));
    setScrollOffset(offset);
  }
}, [scrollToIndex]);
```

### Option B: Imperative Handle via Ref

```typescript
export interface VirtualListHandle {
  scrollToIndex: (index: number) => void;
  getScrollOffset: () => number;
}

const VirtualList = forwardRef<VirtualListHandle, VirtualListProps<T>>((props, ref) => {
  useImperativeHandle(ref, () => ({
    scrollToIndex: (index: number) => {
      setScrollOffset(Math.max(0, Math.min(index, maxScrollOffset)));
    },
    getScrollOffset: () => scrollOffset,
  }));
  // ...
});
```

## Edge Cases to Handle

### 1. Empty Conversation
- When conversation is empty, disable turn selection
- `selectedTurnIndex` should be `null`

### 2. New Messages During Selection
- When a new message arrives while in turn selection mode:
  - Keep current selection if not at bottom
  - Auto-select new message if was at last turn (similar to scroll behavior)

### 3. Very Long Turns (Multi-Screen)
- When a turn spans multiple screens, scroll to show the START of the turn
- Consider showing turn boundaries indicator (optional enhancement)

### 4. Tool Messages
- Tool messages (`role: 'tool'`) should be selectable as their own turns
- Or group with preceding user/assistant message (design decision)

### 5. Deleting/Clearing Conversation
- Reset `selectedTurnIndex` when conversation is cleared
- Adjust selection if turns are removed

## Visual Design

### Selected Turn Appearance

```
┌─ Header ─────────────────────────────────┐
│ Claude (claude-sonnet-4) [SELECT] [25%]  │
├──────────────────────────────────────────┤
│   You: What is the weather?              │  <- Not selected
│   ● I cannot check weather directly.     │  <- Not selected
│ > You: Tell me a joke                    │  <- SELECTED TURN START
│ > about programming                      │  <- SELECTED TURN (cyan)
│ > ● Why do programmers prefer dark mode? │  <- SELECTED TURN
│ > Because light attracts bugs!           │  <- SELECTED TURN END
│   You: Thanks!                           │  <- Not selected
└──────────────────────────────────────────┘
```

### Turn Boundaries (Optional Enhancement)

Could add subtle visual separators between turns:
```
│   You: Message 1                         │
│   ● Response 1                           │
│ ─────────────────────────────────────────│  <- Turn separator
│ > You: Message 2 (SELECTED)              │
│ > ● Response 2 (SELECTED)                │
│ ─────────────────────────────────────────│
│   You: Message 3                         │
```

## Testing Strategy

### Unit Tests

1. **Enable turn selection mode**
   - `/select` enables mode
   - `[SELECT]` indicator appears
   - Last turn is auto-selected

2. **Navigate between turns**
   - Up arrow selects previous turn
   - Down arrow selects next turn
   - Navigation wraps or stops at boundaries

3. **Turn highlighting**
   - All lines in selected turn are cyan
   - `>` prefix on all lines of selected turn
   - Non-selected turns have normal colors

4. **Disable turn selection mode**
   - `/select` again disables mode
   - `[SELECT]` indicator disappears
   - Scroll mode restored

5. **Edge cases**
   - Empty conversation handling
   - Single turn handling
   - New message during selection

### Integration Tests

1. Verify selection persists across re-renders
2. Verify scroll position adjusts to show selected turn
3. Verify tool messages are handled correctly

## Implementation Order

1. **Phase 1: Basic Turn Selection**
   - Add `isTurnSelectMode` and `selectedTurnIndex` state
   - Compute `turnBoundaries` memo
   - Update renderItem for turn-based highlighting
   - Change `/select` command to toggle turn mode

2. **Phase 2: Navigation**
   - Add useInput handler for turn navigation
   - Implement up/down arrow turn navigation
   - Handle edge cases (first/last turn)

3. **Phase 3: Scrolling**
   - Add `scrollToIndex` prop to VirtualList (or imperative handle)
   - Scroll to selected turn on change
   - Handle very long turns

4. **Phase 4: Polish**
   - Visual refinements
   - Edge case handling
   - Update tests

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| VirtualList needs modification | Medium | Keep changes minimal (just `scrollToIndex`) |
| Performance with many turns | Low | turnBoundaries is O(n) but memo'd |
| Key handling conflicts | Medium | Carefully manage isFocused flags |
| Breaking existing scroll behavior | High | Thorough testing of scroll mode |

## Summary

The implementation leverages the existing `messageIndex` field in `ConversationLine` to identify turns. The main work is:

1. **AgentView changes**: New state, navigation handlers, renderItem logic
2. **VirtualList changes**: Add `scrollToIndex` prop for programmatic scrolling
3. **Tests**: 8 scenarios covering enable/disable, navigation, highlighting, edge cases

Estimated effort: 4-6 hours including tests.
