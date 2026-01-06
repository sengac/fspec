# Research: /expand Command for Turn-Based Tool Output Expansion

## Overview

This document captures research findings for implementing the `/expand` command that allows toggling expansion of collapsed tool output when a turn is selected via `/select`.

## Current Implementation Analysis

### Turn Selection System

**File**: `src/tui/components/AgentView.tsx`

The turn selection mode is controlled by:
- **State**: `isTurnSelectMode` (boolean) - toggled via `/select` command (line 595)
- **Handler**: Lines 1191-1209 handle the `/select` command

```typescript
if (userMessage === '/select') {
  setInputValue('');
  const newMode = !isTurnSelectMode;
  setIsTurnSelectMode(newMode);
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

### VirtualList Integration

The `VirtualList` component (line 4769-4950) is configured with:

1. **Selection Mode**: `selectionMode={isTurnSelectMode ? 'item' : 'scroll'}`
2. **Custom Navigation**: `getNextIndex` callback for turn-based navigation (navigates between turns, not individual lines)
3. **Custom Selection**: `getIsSelected` callback to highlight all lines belonging to the same `messageIndex`

```typescript
getIsSelected={isTurnSelectMode ? (index, selectedIndex, items) => {
  if (items.length === 0) return false;
  return items[index]?.messageIndex === items[selectedIndex]?.messageIndex;
} : undefined}
```

### Collapsed Output System

**Location**: Lines 348-376, 441-534

Tool output is collapsed using several functions:

1. **`formatCollapsedOutput`** (line 350): For normal tool output
   - Shows first 4 lines (`COLLAPSED_LINES = 4`)
   - Appends `... +N lines (ctrl+o to expand)`

2. **`formatDiffForDisplay`** (line 441): For Edit/Write tool diffs
   - Shows first 25 lines (`DIFF_COLLAPSED_LINES = 25`)
   - Appends `... +N lines (ctrl+o to expand)`

3. **`formatWithTreeConnectors`** (line 337): Adds tree connector prefix
   - First line: `L content`
   - Subsequent lines: `  content` (indented)

### Content Storage

**Key insight**: The collapsed content (with hint message) is baked into the `conversation` state when tool results are processed.

**Tool Result Handling** (lines 2024-2127):
```typescript
if (pendingDiff) {
  // For Edit/Write tools - diff display
  toolResultContent = formatDiffForDisplay(diffLines, DIFF_COLLAPSED_LINES, startLine);
} else {
  // Normal tool result
  const sanitizedContent = result.content.replace(/\t/g, '  ');
  toolResultContent = formatCollapsedOutput(sanitizedContent);
}
// Content is combined with header and stored:
content: `${headerLine}\n${toolResultContent}`
```

### Data Flow

```
ToolResult event
    ↓
formatCollapsedOutput() or formatDiffForDisplay()
    ↓
conversation state (collapsed content with hint stored)
    ↓
deferredConversation (useDeferredValue for performance)
    ↓
wrapMessageToLines() → conversationLines
    ↓
VirtualList renders lines
```

### Hint Message Locations

The "(ctrl+o to expand)" hint appears in:
1. `formatCollapsedOutput` (line 360)
2. `formatDiffForDisplay` (line 466) - for diffs
3. `formatDiffForDisplayReadTool` (line 530) - for Read tool

**Note**: ctrl+o is NOT actually handled anywhere - it's just a display hint.

## Implementation Design

### Data Model Changes

#### 1. ConversationMessage Interface (around line 287)

Add optional `fullContent` field:
```typescript
interface ConversationMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  fullContent?: string;  // NEW: Full uncollapsed content for expandable messages
  isStreaming?: boolean;
}
```

#### 2. New State

Add expanded messages tracking:
```typescript
const [expandedMessageIndices, setExpandedMessageIndices] = useState<Set<number>>(new Set());
```

### New Functions

#### 1. formatFullOutput (non-collapsed version)

```typescript
const formatFullOutput = (content: string): string => {
  return formatWithTreeConnectors(content);
};
```

#### 2. formatDiffForDisplayFull (non-collapsed diff)

Create a variant that shows all lines without the "...+N lines" truncation.

### Modified Functions

#### 1. Tool Result Handler

Store both collapsed and full content:
```typescript
const sanitizedContent = result.content.replace(/\t/g, '  ');
const toolResultContent = formatCollapsedOutput(sanitizedContent);
const fullToolResultContent = formatFullOutput(sanitizedContent);

// When updating message:
{
  content: `${headerLine}\n${toolResultContent}`,
  fullContent: `${headerLine}\n${fullToolResultContent}`,
}
```

#### 2. conversationLines Computation (line 4059)

Check expanded state and swap content:
```typescript
deferredConversation.forEach((msg, msgIndex) => {
  // Use fullContent if message is expanded
  const effectiveContent = expandedMessageIndices.has(msgIndex) && msg.fullContent
    ? msg.fullContent
    : msg.content;
  
  const effectiveMsg = { ...msg, content: effectiveContent };
  // ... rest of processing
});
```

### New /expand Command Handler

```typescript
if (userMessage === '/expand') {
  setInputValue('');
  
  if (!isTurnSelectMode) {
    setConversation(prev => [
      ...prev,
      {
        role: 'tool',
        content: 'Error: Must be in turn selection mode. Use /select first.',
      },
    ]);
    return;
  }
  
  // Get currently selected message index from VirtualList
  // Need to expose selectedIndex or use a ref
  const selectedMessageIndex = getSelectedMessageIndex(); // Implementation needed
  
  if (selectedMessageIndex === undefined) {
    setConversation(prev => [
      ...prev,
      { role: 'tool', content: 'Error: No turn selected.' },
    ]);
    return;
  }
  
  // Toggle expansion
  setExpandedMessageIndices(prev => {
    const newSet = new Set(prev);
    if (newSet.has(selectedMessageIndex)) {
      newSet.delete(selectedMessageIndex);
    } else {
      newSet.add(selectedMessageIndex);
    }
    return newSet;
  });
  
  return;
}
```

### Getting Selected Message Index

The `selectedIndex` is internal to VirtualList. Options:

1. **Use a callback prop**: Add `onSelectionChange` to VirtualList that reports the selected index
2. **Use a ref**: Pass a ref to VirtualList that it updates with current selection
3. **Store in parent state**: Lift `selectedIndex` state to AgentView

**Recommended**: Option 2 (ref) - least invasive change to VirtualList.

```typescript
// In AgentView:
const virtualListSelectionRef = useRef<{ selectedIndex: number }>({ selectedIndex: 0 });

// Pass to VirtualList:
<VirtualList
  selectionRef={virtualListSelectionRef}
  ...
/>

// In /expand handler:
const selectedLineIndex = virtualListSelectionRef.current.selectedIndex;
const selectedMessageIndex = conversationLines[selectedLineIndex]?.messageIndex;
```

### Hint Message Update

Change from:
```
... +N lines (ctrl+o to expand)
```

To:
```
... +N lines (use /select and /expand)
```

Locations to update:
- `formatCollapsedOutput` (line 360)
- `formatDiffForDisplay` (line 466)
- `formatDiffForDisplayReadTool` (line 530)

### Cache Invalidation

The `lineCacheRef` caches computed lines per message. When expansion state changes, the cache entry for that message needs to be invalidated.

```typescript
// When toggling expansion:
setExpandedMessageIndices(prev => {
  const newSet = new Set(prev);
  // ... toggle logic
  
  // Invalidate cache for this message
  lineCacheRef.current.delete(selectedMessageIndex);
  
  return newSet;
});
```

## Edge Cases

1. **No expandable content**: If the selected turn has no collapsed content (e.g., user message, short tool output), `/expand` should be a no-op or show a message.

2. **Multiple tool outputs in one turn**: A single turn (messageIndex) could have multiple tool calls. The expansion applies to the entire turn.

3. **Streaming in progress**: Should `/expand` work while a tool is still streaming? Probably not - wait until stream completes.

4. **Selection changes**: When navigating to a different turn, the expanded state for the previous turn is preserved (user can expand multiple turns).

5. **Clear session**: When `/clear` is called, reset `expandedMessageIndices` to empty set.

## Testing Considerations

1. **Unit test**: Toggle expansion state correctly
2. **Unit test**: Full content stored correctly when collapsing
3. **Unit test**: Cache invalidation on expansion toggle
4. **Integration test**: /expand command flow
5. **Visual test**: Expanded content renders correctly with tree connectors

## Files to Modify

1. `src/tui/components/AgentView.tsx`
   - Add `expandedMessageIndices` state
   - Add `fullContent` to ConversationMessage
   - Modify tool result handlers to store fullContent
   - Add `/expand` command handler
   - Modify conversationLines computation
   - Update hint messages

2. `src/tui/components/VirtualList.tsx`
   - Add `selectionRef` prop to expose current selection

## Summary

The implementation requires:
1. Storing full uncollapsed content alongside collapsed content
2. Tracking which messages are expanded
3. Conditionally using full content when rendering expanded messages
4. Exposing VirtualList's selection state to parent
5. Adding the `/expand` command handler
6. Updating hint messages to reference the new command
