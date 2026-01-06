# TUI-045: Select Mode Enter Key Opens Full Turn Modal

## Summary

This story replaces the current inline `/expand` command with a modal-based approach for viewing full turn content. When in select mode (Tab key), pressing Enter will open a modal dialog showing the selected turn's full content in a scrollable VirtualList.

## Current Implementation Analysis

### Select Mode (TUI-042)

**Location**: `src/tui/components/AgentView.tsx`

**Current behavior**:
- Tab key toggles `isTurnSelectMode` state (line 4042-4052)
- When enabled, `[SELECT]` indicator appears in status bar (line 4855-4861)
- VirtualList switches to `selectionMode="item"` with group-based selection by `messageIndex` (line 5024-5028)
- Arrow keys navigate between turns (groups), highlighted with gray background arrow bars
- Currently, Esc clears input → interrupts loading → exits agent view (line 4028-4039)

### Expand Command (TUI-043) - TO BE REMOVED

**Current behavior**:
- `/expand` command toggles `expandedMessageIndices` state (line 1249-1281)
- When a message index is in `expandedMessageIndices`, `fullContent` is used instead of `content` (line 4188-4190)
- This causes inline expansion within the conversation view
- Cache invalidation required when toggling (line 1269)

**Files to remove/modify**:
- Remove `/expand` command handler (lines 1249-1281)
- Remove `expandedMessageIndices` state (line 605)
- Remove expansion logic in `conversationLines` useMemo (lines 4188-4190)
- Keep `fullContent` field on ConversationMessage - needed for modal

### ConversationMessage Interface

```typescript
interface ConversationMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;           // Collapsed/truncated content for main view
  fullContent?: string;      // Full uncollapsed content for modal view
  isStreaming?: boolean;
}
```

### Existing Modal Pattern

**Reference**: `src/components/Dialog.tsx` and `src/tui/components/AttachmentDialog.tsx`

The Dialog component provides:
- Centered modal overlay with border
- Esc key handling via `useInput`
- `isActive` prop to control input capture
- Black background with rounded border

## Implementation Requirements

### New State

```typescript
const [showTurnModal, setShowTurnModal] = useState(false);
const [modalContent, setModalContent] = useState<string>('');
const [modalRole, setModalRole] = useState<'user' | 'assistant' | 'tool'>('assistant');
```

### New Component: TurnContentModal

Create a new component or inline modal that:
1. Uses Dialog base component for overlay
2. Contains VirtualList for scrollable content
3. Wraps content lines for terminal width
4. Preserves diff coloring (red/green backgrounds) for tool output
5. Shows role indicator in title (e.g., "Assistant Response" or "Your Message")

### Key Bindings Changes

#### Enter Key in Select Mode
- When `isTurnSelectMode && !showTurnModal`:
  - Get selected turn via `virtualListSelectionRef.current.selectedIndex`
  - Look up `conversation[messageIndex]`
  - Set `modalContent` to `fullContent || content`
  - Set `modalRole` to message role
  - Set `showTurnModal = true`

#### Esc Key Behavior (Priority Order)
1. If `showTurnModal` → close modal (`setShowTurnModal(false)`)
2. If `isTurnSelectMode` → disable select mode (`setIsTurnSelectMode(false)`)
3. If `isLoading` → interrupt agent
4. If `inputValue` not empty → clear input
5. Else → exit agent view

### VirtualList Configuration for Modal

```typescript
<VirtualList
  items={modalLines}
  renderItem={renderModalLine}  // Same coloring logic as main view
  isFocused={showTurnModal}
  scrollToEnd={false}           // Start at top
  selectionMode="scroll"        // No item selection in modal
  showScrollbar={true}
/>
```

### Content Processing

The modal should display `fullContent` which already contains:
- Full diff output without truncation (for Edit/Write tools)
- All lines visible (no "... +N lines" indicators)
- Tree connectors (L prefix) preserved

The rendering in modal needs to:
1. Split content by newlines
2. Wrap lines for terminal width
3. Apply same diff coloring logic as main view:
   - `[R]` marker → red background
   - `[A]` marker → green background
   - Context lines → gray line numbers, white content

## Files to Modify

1. **`src/tui/components/AgentView.tsx`**:
   - Add `showTurnModal`, `modalContent`, `modalRole` state
   - Modify Esc key handler for modal-first closing
   - Add Enter key handler in select mode
   - Remove `/expand` command handler
   - Remove `expandedMessageIndices` state
   - Remove expansion logic from `conversationLines` useMemo
   - Add modal component JSX (either inline or new component)

2. **Optional: `src/tui/components/TurnContentModal.tsx`**:
   - New component if extracting modal logic
   - Would receive content, role, onClose props
   - Contains VirtualList with diff rendering

## UI/UX Details

### Modal Appearance
- Full terminal width minus 4 chars for border
- Height: terminal height minus 6 lines (for borders + title)
- Title bar: "[Role] Turn Content" (e.g., "Assistant Turn Content")
- Footer: "↑↓ Scroll | Esc Close"
- Scrollbar on right side

### Status Bar
- When modal open, could show `[VIEWING]` instead of `[SELECT]`
- Or just keep `[SELECT]` indicator

## Edge Cases

1. **Turn with no fullContent**: Use `content` field instead
2. **User turn (role='user')**: Show full user message
3. **Tool turn (role='tool')**: Show full tool output with diff colors
4. **Empty content**: Show "No content" message
5. **Very long single line**: Wrap using same word-wrap logic as main view

## Testing Considerations

1. Enter opens modal with correct content
2. Esc closes modal, returns to select mode
3. Esc again disables select mode
4. Diff coloring preserved in modal
5. Scrolling works in modal
6. Modal content matches selected turn
7. `/expand` command no longer recognized

## Related Work Units

- **TUI-042**: Turn selection mode (Tab key toggle)
- **TUI-043**: Expandable message content (being replaced)
- **TUI-044**: Group-based selection preservation
