# AST Research: Turn Selection Integration Points

## Overview

This research document identifies the key integration points for implementing turn-based selection in AgentView, replacing the current line-based selection from TUI-041.

## 1. Current State Analysis (TUI-041)

### isLineSelectMode State Location
```
src/tui/components/AgentView.tsx:594:10:isLineSelectMode
src/tui/components/AgentView.tsx:1194:24:isLineSelectMode (in /select handler)
src/tui/components/AgentView.tsx:4720:12:isLineSelectMode ([SELECT] indicator)
src/tui/components/AgentView.tsx:4850:26:isLineSelectMode (VirtualList selectionMode prop)
```

**Analysis**: 4 locations to modify - state declaration, command handler, header indicator, and VirtualList prop.

### useState Declarations in AgentView (for reference)
```
src/tui/components/AgentView.tsx:581:39:useState('') - inputValue
src/tui/components/AgentView.tsx:582:37:useState(false) - isLoading
src/tui/components/AgentView.tsx:591:59:useState(false) - showProviderSelector
src/tui/components/AgentView.tsx:592:61:useState(0) - selectedProviderIndex
src/tui/components/AgentView.tsx:593:47:useState(false) - showModelSelector
src/tui/components/AgentView.tsx:594:51:useState(false) - isLineSelectMode ← REPLACE
... (28 total useState calls)
```

**Action**: Add `selectedTurnIndex` state near line 594.

## 2. VirtualList Integration

### VirtualList Props Interface
```
src/tui/components/VirtualList.tsx:88:1:interface VirtualListProps<T>
```

### selectionMode Usage in VirtualList
```
src/tui/components/VirtualList.tsx:234:11:selectionMode - scrollToEnd effect
src/tui/components/VirtualList.tsx:241:49:selectionMode - dependency array
src/tui/components/VirtualList.tsx:246:9:selectionMode - ensureSelectedVisible
src/tui/components/VirtualList.tsx:253:51:selectionMode - dependency array
src/tui/components/VirtualList.tsx:258:9:selectionMode - onFocus effect
src/tui/components/VirtualList.tsx:267:38:selectionMode - dependency array
src/tui/components/VirtualList.tsx:313:11:selectionMode - handleScrollNavigation
src/tui/components/VirtualList.tsx:334:35:selectionMode - dependency array
src/tui/components/VirtualList.tsx:452:173:selectionMode - useInput logging
src/tui/components/VirtualList.tsx:471:11:selectionMode - useInput handler
src/tui/components/VirtualList.tsx:497:13:selectionMode - isSelected check
```

**Action**: Add `scrollToIndex?: number` prop to VirtualListProps interface.

## 3. ConversationLine Interface

### messageIndex Field (grep results)
```
src/tui/components/AgentView.tsx:298:  messageIndex: number;
src/tui/components/AgentView.tsx:3962:  { role: msg.role, content: ' ', messageIndex: msgIndex }
src/tui/components/AgentView.tsx:3981:  messageIndex: msgIndex
src/tui/components/AgentView.tsx:3995:  messageIndex: msgIndex
src/tui/components/AgentView.tsx:4018:  messageIndex: msgIndex
src/tui/components/AgentView.tsx:4035:  messageIndex: msgIndex
src/tui/components/AgentView.tsx:4039:  { role: msg.role, content: ' ', messageIndex: msgIndex }
src/tui/components/AgentView.tsx:4045:  { role: msg.role, content: ' ', messageIndex: msgIndex }
```

**Key Insight**: Every ConversationLine already has `messageIndex` - this IS the turn index. No new field needed.

## 4. useInput Handler Location

### Current useInput in AgentView
```
src/tui/components/AgentView.tsx:3427:  useInput(
```

**Action**: Add a second useInput handler for turn navigation when isTurnSelectMode is true.

## 5. Implementation Changes Summary

### AgentView.tsx Changes

| Location | Current | Change To |
|----------|---------|-----------|
| Line 594 | `isLineSelectMode` | Rename to `isTurnSelectMode` |
| Line 594+ | - | Add `selectedTurnIndex` state |
| Line 594+ | - | Add `turnBoundaries` useMemo |
| Line 1194 | `/select` handler | Update to set selectedTurnIndex |
| Line 3427+ | - | Add turn navigation useInput |
| Line 4720 | `isLineSelectMode &&` | `isTurnSelectMode &&` |
| Line 4850 | `selectionMode={...}` | Always 'scroll' when turn mode active |
| Line 4848+ | - | Set `isFocused={false}` when turn mode |
| Line 4765+ | `renderItem` | Check `line.messageIndex === selectedTurnIndex` |

### VirtualList.tsx Changes

| Location | Change |
|----------|--------|
| Line 88+ | Add `scrollToIndex?: number` to interface |
| After 230 | Add useEffect for scrollToIndex |

## 6. turnBoundaries Memo Structure

```typescript
const turnBoundaries = useMemo(() => {
  const boundaries: Array<{
    turnIndex: number;
    firstLineIndex: number;
    lastLineIndex: number;
  }> = [];
  
  let currentTurn = -1;
  let firstLine = 0;
  
  conversationLines.forEach((line, idx) => {
    if (line.messageIndex !== currentTurn) {
      if (currentTurn >= 0) {
        boundaries.push({
          turnIndex: currentTurn,
          firstLine,
          lastLine: idx - 1
        });
      }
      currentTurn = line.messageIndex;
      firstLine = idx;
    }
  });
  
  if (currentTurn >= 0 && conversationLines.length > 0) {
    boundaries.push({
      turnIndex: currentTurn,
      firstLine,
      lastLine: conversationLines.length - 1
    });
  }
  
  return boundaries;
}, [conversationLines]);
```

## 7. Risk Assessment

| Risk | Mitigation |
|------|------------|
| VirtualList prop addition | Minimal change, additive only |
| State rename refactoring | 4 simple location changes |
| Turn navigation conflicts | isFocused=false disables VL input |
| Scroll position sync | scrollToIndex prop handles this |

## 8. Files to Modify

1. `src/tui/components/AgentView.tsx` - Primary changes
2. `src/tui/components/VirtualList.tsx` - Add scrollToIndex prop
3. `src/tui/__tests__/AgentView-select-mode.test.tsx` - Update tests for turn-based selection

## Research Conducted

- AST pattern search for useState declarations
- AST pattern search for selectionMode usage
- AST pattern search for isLineSelectMode
- grep search for messageIndex field usage
- grep search for useInput locations
- Manual code review of VirtualListProps interface
