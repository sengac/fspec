# TUI-045: Select Mode Enter Key Opens Full Turn Modal

## Summary

This story replaces the current inline `/expand` command with a modal-based approach for viewing full turn content. When in select mode (Tab key), pressing Enter will open a modal dialog showing the selected turn's full content in a scrollable VirtualList.

## User Story

**As a** user in select mode navigating turns in AgentView  
**I want to** press Enter to open the selected turn in a full-screen scrollable modal, and use Esc to close the modal or exit select mode  
**So that** I can view large turns (with long tool outputs) in their entirety in a dedicated modal view, replacing the inline /expand command

---

## Business Rules

1. Pressing Enter in select mode opens the selected turn's full content (fullContent field) in a modal dialog
2. The modal displays the turn content in a scrollable VirtualList (not collapsed/truncated)
3. Pressing Esc while the modal is open closes the modal and returns to select mode
4. Pressing Esc while in select mode (and modal is NOT open) disables select mode
5. The /expand command is removed - Enter key in modal view replaces it entirely
6. The expandedMessageIndices state is removed - no longer needed for inline expansion
7. VirtualList onSelect callback is used to open the modal when Enter is pressed in select mode
8. Modal state consists of showTurnModal boolean and modalMessageIndex number
9. Main VirtualList isFocused must include `&& !showTurnModal` to prevent input capture when modal is open
10. The collapse hint text changes from `(select turn to /expand)` to `(Enter to view full)`
11. Modal VirtualList uses `selectionMode='scroll'` with `scrollToEnd=false`
12. Esc key priority order: 1) close modal, 2) disable select mode, 3) interrupt loading, 4) clear input, 5) exit view
13. Diff coloring logic is shared between main view and modal via extracted renderConversationLine function
14. Modal content is prepared by wrapping (fullContent || content) into ConversationLine[] with word-wrapping

---

## Architecture Notes

### 1. VirtualList onSelect Integration
Use existing VirtualList onSelect prop rather than custom Enter key handler. VirtualList already calls onSelect on Enter in item mode (line 473-474 in VirtualList.tsx).

```typescript
// In main VirtualList
onSelect={(line, index) => {
  const msgIndex = line.messageIndex;
  setModalMessageIndex(msgIndex);
  setShowTurnModal(true);
}}
```

### 2. Extract renderConversationLine Function
Extract the inline renderItem logic from the main VirtualList to share diff coloring logic:

```typescript
const renderConversationLine = (line: ConversationLine, terminalWidth: number): React.ReactNode => {
  const content = line.content;
  
  // Diff coloring: [R] for removed, [A] for added
  if (line.role === 'tool') {
    const rIdx = content.indexOf('[R]');
    const aIdx = content.indexOf('[A]');
    // ... existing diff coloring logic
  }
  
  // Default rendering
  const baseColor = line.role === 'user' ? 'green' : 'white';
  return <Text color={baseColor}>{content}</Text>;
};
```

### 3. Modal Positioning
Use Dialog component pattern with position='absolute' width='100%' height='100%' for full-screen overlay:

```typescript
<Box
  position="absolute"
  width="100%"
  height="100%"
  justifyContent="center"
  alignItems="center"
>
  <Box
    flexDirection="column"
    borderStyle="round"
    padding={1}
    backgroundColor="black"
    width={terminalWidth - 4}
    height={terminalHeight - 6}
  >
    {/* Modal content */}
  </Box>
</Box>
```

### 4. Esc Handler Modification
Modify existing Esc handler (line 4028-4039) to check showTurnModal and isTurnSelectMode BEFORE existing checks:

```typescript
if (key.escape) {
  // NEW: Priority 1 - Close modal first
  if (showTurnModal) {
    setShowTurnModal(false);
    return;
  }
  // NEW: Priority 2 - Disable select mode
  if (isTurnSelectMode) {
    setIsTurnSelectMode(false);
    return;
  }
  // EXISTING: Priority 3 - Interrupt loading
  if (isLoading && sessionRef.current) {
    sessionRef.current.interrupt();
  } 
  // EXISTING: Priority 4 - Clear input
  else if (inputValue.trim() !== '') {
    setInputValue('');
  } 
  // EXISTING: Priority 5 - Exit view
  else {
    onExit();
  }
  return;
}
```

### 5. Remove /expand Command
Remove /expand command handler block (lines 1249-1281) and expandedMessageIndices state (line 605) and its usage in conversationLines useMemo.

### 6. Create modalLines useMemo
Create modalLines that wraps (fullContent || content) for modalMessageIndex:

```typescript
const modalLines = useMemo((): ConversationLine[] => {
  if (!showTurnModal || modalMessageIndex === null) return [];
  
  const msg = conversation[modalMessageIndex];
  if (!msg) return [];
  
  const content = msg.fullContent || msg.content;
  const maxWidth = terminalWidth - 8; // Account for modal borders
  
  // Word-wrap content into lines (similar to wrapMessageToLines but no prefix/separator)
  return wrapContentToLines(content, msg.role, modalMessageIndex, maxWidth);
}, [showTurnModal, modalMessageIndex, conversation, terminalWidth]);
```

### 7. Update formatCollapsedOutput
Change hint from `(select turn to /expand)` to `(Enter to view full)`:

```typescript
const formatCollapsedOutput = (content: string, visibleLines: number = COLLAPSED_LINES): string => {
  const lines = content.split('\n');
  if (lines.length <= visibleLines) {
    return formatWithTreeConnectors(content);
  }
  const visible = lines.slice(0, visibleLines);
  const remaining = lines.length - visibleLines;
  // CHANGED: Updated hint text
  const collapsedContent = `${visible.join('\n')}\n... +${remaining} lines (Enter to view full)`;
  return formatWithTreeConnectors(collapsedContent);
};
```

### 8. Update isFocused Prop
Update main VirtualList isFocused prop to include `&& !showTurnModal`:

```typescript
<VirtualList
  // ...
  isFocused={!showProviderSelector && !showModelSelector && !showSettingsTab && !isResumeMode && !isSearchMode && !showTurnModal}
  // ...
/>
```

### 9. Modal Keyboard Handling
Use main useInput with priority check (showTurnModal checked first in Esc handler). No separate useInput needed for modal.

---

## New State Variables

```typescript
const [showTurnModal, setShowTurnModal] = useState(false);
const [modalMessageIndex, setModalMessageIndex] = useState<number | null>(null);
```

---

## Modal UI Structure

```
┌─────────────────────────────────────────────┐
│ [Role] Turn Content                         │  ← Title (role-specific)
├─────────────────────────────────────────────┤
│                                             │
│  (scrollable VirtualList with full content) │
│                                             │
│  L first line of output                     │
│    second line                              │
│    third line                               │
│    ... (all lines, no truncation)           │
│                                             │
├─────────────────────────────────────────────┤
│ ↑↓ Scroll | Esc Close                       │  ← Footer
└─────────────────────────────────────────────┘
```

**Title based on role:**
- `role === 'user'` → "User Message"
- `role === 'assistant'` → "Assistant Response"
- `role === 'tool'` → "Tool Output"

---

## Files to Modify

### Primary: `src/tui/components/AgentView.tsx`

1. **Add new state** (after line 605):
   ```typescript
   const [showTurnModal, setShowTurnModal] = useState(false);
   const [modalMessageIndex, setModalMessageIndex] = useState<number | null>(null);
   ```

2. **Remove old state** (line 605):
   ```typescript
   // DELETE: const [expandedMessageIndices, setExpandedMessageIndices] = useState<Set<number>>(new Set());
   ```

3. **Remove /expand handler** (lines 1249-1281)

4. **Extract renderConversationLine** (before main VirtualList, ~line 4890)

5. **Add modalLines useMemo** (after conversationLines useMemo)

6. **Modify Esc handler** (line 4028-4039) - add showTurnModal and isTurnSelectMode checks

7. **Add onSelect to main VirtualList** (~line 5018)

8. **Update isFocused prop** (~line 5022)

9. **Update formatCollapsedOutput** (line 352-364)

10. **Add modal JSX** (before closing `</Box>` of main view)

11. **Remove expandedMessageIndices from conversationLines useMemo deps** (~line 4226)

---

## Scenarios Coverage

| # | Scenario | Covers Rule(s) | Covers Example(s) |
|---|----------|----------------|-------------------|
| 1 | Open truncated tool output in modal | [0], [1] | [0] |
| 2 | Close modal with Esc returns to select mode | [2] | [1] |
| 3 | Exit select mode with Esc when modal is not open | [3] | [2] |
| 4 | Open user message in modal | [0] | [3] |
| 5 | Diff coloring preserved in modal | [12] | [4] |
| 6 | Esc behavior when not in select mode | [11] | [5] |
| 7 | Modal displays role-specific title (Outline) | - | [6] |
| 8 | Modal displays navigation hints in footer | - | [7] |
| 9 | Open short message with no fullContent | - | [8] |
| 10 | Collapsed output shows updated hint text | [9] | [9] |
| 11 | Esc key closes modal before disabling select mode | [11] | - |
| 12 | The /expand command is no longer recognized | [4] | - |

---

## Related Work Units

- **TUI-042**: Turn selection mode (Tab key toggle) - provides the select mode infrastructure
- **TUI-043**: Expandable message content (being replaced by this story)
- **TUI-044**: Group-based selection preservation - ensures selection persists correctly
