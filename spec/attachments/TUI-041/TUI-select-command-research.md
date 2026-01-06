# Research Notes: /select Command for Line Selection Mode Toggle

## Overview

This document contains comprehensive research notes for implementing a `/select` command in AgentView that toggles between line selection mode and pure viewport scrolling mode in the conversation VirtualList.

## Current State Analysis

### VirtualList Selection Modes (TUI-032)

The VirtualList component (`src/tui/components/VirtualList.tsx`) supports two selection modes via the `selectionMode` prop:

```typescript
selectionMode?: 'item' | 'scroll'; // 'item' = individual item selection (default), 'scroll' = pure viewport scrolling (TUI-032)
```

#### Mode 1: `selectionMode="item"` (Default)
- Arrow keys navigate between items with visual selection indicator
- Selected item tracked with `selectedIndex` state
- `isSelected` passed as `true` to renderItem for the currently selected item
- `onFocus` callback fires when selection changes
- Scroll offset auto-adjusts to keep selected item visible
- Used by: CheckpointViewer, FileDiffViewer, ChangedFilesViewer

#### Mode 2: `selectionMode="scroll"` (Current AgentView Behavior)
- Arrow keys only scroll the viewport, no item selection
- `isSelected` is **always false** for all items (line 495-497)
- `onFocus` is **never called** (lines 257-267)
- Supports `scrollToEnd` prop for chat-style auto-scroll behavior
- Currently used by: AgentView conversation VirtualList

### Smart Scrolling System

The VirtualList implements a sophisticated "smart scrolling" system with sticky scroll behavior:

#### Key State Variables (lines 133-147)
```typescript
const [selectedIndex, setSelectedIndex] = useState(0);
const [scrollOffset, setScrollOffset] = useState(0);
const lastScrollTime = useRef<number>(0);
const scrollVelocity = useRef<number>(1);
const [userScrolledAway, setUserScrolledAway] = useState(false);
```

#### Sticky Scroll Behavior (`scrollToEnd` + `userScrolledAway`)

1. **Auto-scroll to end** (lines 227-241):
   - When `scrollToEnd=true` and new items are added
   - Only auto-scrolls if `!userScrolledAway`
   - In scroll mode: only updates `scrollOffset`
   - In item mode: also updates `selectedIndex`

2. **User scroll tracking** (lines 318-328, 410-418):
   - When user scrolls UP and not at bottom: `setUserScrolledAway(true)`
   - When user scrolls to bottom (within 1 line): `setUserScrolledAway(false)`
   - This "breaks" auto-scroll when user manually scrolls away
   - Re-enables auto-scroll when user returns to bottom

3. **Scroll acceleration** (lines 296-335):
   - Mouse wheel and rapid scrolling accelerate (up to 5 items per scroll)
   - Reset to 1 item after 150ms pause for precise control

### Navigation Handlers

Two separate navigation handlers exist (lines 378-446):

#### `handleScrollNavigation` (scroll mode)
- Directly manipulates `scrollOffset`
- Tracks `userScrolledAway` for sticky scroll behavior
- Keys: up/down (±1), pageUp/pageDown (±visibleHeight), home/end (0/max)

#### `handleItemNavigation` (item mode)
- Uses `navigateTo()` which updates `selectedIndex`
- Scroll offset auto-adjusts to keep selected item visible
- Keys: same as above, plus Enter for `onSelect`

### Command Parsing in AgentView

Commands like `/debug`, `/clear`, `/resume`, `/search` are handled in the `handleSubmit` callback (lines 1188-1317):

```typescript
const handleSubmit = useCallback(async () => {
  if (!sessionRef.current || !inputValue.trim() || isLoading) return;
  const userMessage = inputValue.trim();

  // AGENT-021: Handle /debug command - toggle debug capture mode
  if (userMessage === '/debug') {
    setInputValue('');
    try {
      const result = sessionRef.current.toggleDebug(getFspecUserDir());
      setIsDebugEnabled(result.enabled);
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: result.message },
      ]);
    } catch (err) { /* ... */ }
    return;
  }

  // Similar pattern for /search, /clear, /resume, etc.
});
```

### Current AgentView VirtualList Usage

The conversation VirtualList in AgentView (lines 4737-4816):

```typescript
<VirtualList
  items={conversationLines}
  renderItem={line => {
    // Rendering logic - isSelected is always false
    const color = line.role === 'user' ? 'green' : 'white';
    return (
      <Box flexGrow={1}>
        <Text color={color}>{content}</Text>
      </Box>
    );
  }}
  keyExtractor={(_line, index) => `line-${index}`}
  emptyMessage=""
  showScrollbar={true}
  isFocused={!showProviderSelector && !showModelSelector && !showSettingsTab && !isResumeMode && !isSearchMode}
  scrollToEnd={true}
  selectionMode="scroll"
/>
```

### UI Indicators Pattern

The DEBUG indicator pattern (lines 4694-4699) shows how toggle states are displayed:

```typescript
{isDebugEnabled && (
  <Text color="red" bold>
    {' '}
    [DEBUG]
  </Text>
)}
```

## Implementation Requirements

### 1. State Variable

Add new state variable to AgentView:
```typescript
const [isLineSelectMode, setIsLineSelectMode] = useState(false);
```

### 2. /select Command Handler

Add to `handleSubmit` after other command handlers:
```typescript
// Handle /select command - toggle line selection mode
if (userMessage === '/select') {
  setInputValue('');
  const newMode = !isLineSelectMode;
  setIsLineSelectMode(newMode);
  setConversation(prev => [
    ...prev,
    { 
      role: 'tool', 
      content: `Line selection mode ${newMode ? 'enabled' : 'disabled'}. ${
        newMode 
          ? 'Arrow keys now select individual lines. Press Enter to copy selected line.'
          : 'Arrow keys now scroll the viewport.'
      }`
    },
  ]);
  return;
}
```

### 3. VirtualList selectionMode Toggle

Update the VirtualList to use dynamic selectionMode:
```typescript
<VirtualList
  items={conversationLines}
  selectionMode={isLineSelectMode ? 'item' : 'scroll'}
  scrollToEnd={true}
  // ... other props
/>
```

### 4. UI Indicator

Add indicator near DEBUG indicator:
```typescript
{isLineSelectMode && (
  <Text color="cyan" bold>
    {' '}
    [SELECT]
  </Text>
)}
```

### 5. Selection Visual Feedback

Update renderItem to show selection in item mode:
```typescript
renderItem={(line, index, isSelected) => {
  // In item mode (isLineSelectMode), show selection
  const baseColor = line.role === 'user' ? 'green' : 'white';
  const color = isSelected ? 'cyan' : baseColor;
  const prefix = isSelected ? '> ' : '  ';
  return (
    <Box flexGrow={1}>
      <Text color={color}>{prefix}{content}</Text>
    </Box>
  );
}}
```

## Smart Scrolling Behavior - IMPORTANT DIFFERENCE BETWEEN MODES

### Critical Discovery: userScrolledAway is NOT Tracked in Item Mode

After careful analysis of the VirtualList code, I found a critical difference:

1. **Scroll mode**: Has "sticky scroll" via `userScrolledAway` tracking
   - `handleScrollNavigation` (lines 378-419) tracks `userScrolledAway`
   - User scrolling up sets `userScrolledAway=true`, disabling auto-scroll
   - Scrolling back to bottom re-enables auto-scroll
   - This allows users to read older messages without being interrupted

2. **Item mode**: Does NOT have sticky scroll
   - `handleItemNavigation` (lines 421-446) does NOT track `userScrolledAway`
   - New messages will ALWAYS auto-select the last line
   - This is INTENTIONAL - item mode is for quick line selection/copy operations

### Why This Design Makes Sense

Looking at the auto-scroll effect (lines 227-241):
```typescript
useEffect(() => {
  if (scrollToEnd && items.length > 0 && !userScrolledAway) {
    const lastIndex = items.length - 1;
    if (selectionMode === 'item') {
      setSelectedIndex(lastIndex);  // Auto-select last line
    }
    const newOffset = Math.max(0, lastIndex - visibleHeight + 1);
    setScrollOffset(newOffset);
  }
}, [scrollToEnd, items.length, visibleHeight, selectionMode, userScrolledAway]);
```

In item mode, since `userScrolledAway` is never set to `true`, the condition `!userScrolledAway` is always true. This means new messages will always trigger auto-selection of the last line.

**This is the INTENDED behavior for the /select feature:**
- **Scroll mode (default)**: For reading conversations - sticky scroll lets you read without interruption
- **Line selection mode**: For quick copy operations - auto-select helps you see new content immediately

### No VirtualList Changes Needed

The current VirtualList behavior is correct for this feature:
- Scroll mode: Sticky scroll works as expected
- Item mode: Always auto-selects last line on new messages (no sticky scroll)

This means we only need to modify AgentView.tsx - no changes to VirtualList.tsx are required.

## Potential Additional Features

### Copy Selected Line

When in line selection mode, Enter key could copy the selected line to clipboard:

```typescript
// In VirtualList handleItemNavigation
if (key.return && onSelect) {
  onSelect(items[selectedIndex], selectedIndex);
}
```

Then in AgentView:
```typescript
<VirtualList
  onSelect={(line, index) => {
    if (isLineSelectMode) {
      // Copy to clipboard
      const clipboardy = await import('clipboardy');
      await clipboardy.default.write(line.content);
      setConversation(prev => [
        ...prev,
        { role: 'tool', content: `Copied line ${index + 1} to clipboard` },
      ]);
    }
  }}
/>
```

### Multi-line Selection (Future Enhancement)

Could add shift+arrow to extend selection for copying multiple lines. This would require additional state:
```typescript
const [selectionStart, setSelectionStart] = useState<number | null>(null);
const [selectionEnd, setSelectionEnd] = useState<number | null>(null);
```

## Testing Considerations

### Unit Tests

1. `/select` command toggles state correctly
2. VirtualList receives correct `selectionMode` prop based on state
3. UI indicator appears/disappears with state
4. Selection visual feedback shows in item mode

### Integration Tests

1. Toggle from scroll to item mode, verify line selection works
2. Toggle back to scroll mode, verify no selection highlighting
3. Auto-scroll behavior in both modes
4. User scroll-away behavior (sticky scroll) in scroll mode

### Manual Testing

1. Start AgentView in scroll mode (default)
2. Scroll through conversation with arrow keys - no selection
3. Type `/select` to enable line selection mode
4. Verify [SELECT] indicator appears
5. Navigate with arrows - selection visible
6. New messages arrive - verify auto-scroll/select
7. Type `/select` again to disable
8. Verify back to scroll-only mode

## Files to Modify

1. `src/tui/components/AgentView.tsx`:
   - Add `isLineSelectMode` state
   - Add `/select` command handler
   - Add UI indicator
   - Update VirtualList `selectionMode` prop
   - Update `renderItem` for selection visual

2. `src/tui/components/VirtualList.tsx`:
   - NO CHANGES NEEDED - existing code handles both modes correctly

## Summary

The implementation is straightforward because VirtualList already supports both modes with the correct behavior:
- Scroll mode: Sticky scroll via `userScrolledAway` tracking
- Item mode: Always auto-selects last line (intentional for quick copy operations)

The main work is:

1. Adding state and command handler to AgentView (~20 lines)
2. Adding UI indicator (~5 lines)
3. Updating VirtualList props and renderItem (~10 lines)
4. Writing tests (~100 lines)

Total estimated effort: ~135 lines of code changes, 3 points
