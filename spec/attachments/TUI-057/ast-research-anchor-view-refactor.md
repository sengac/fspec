# AST Research: Anchor View Refactor (TUI-057)

## Research Summary

Analysis of code structure for refactoring AnchorViewerDialog to full-screen AnchorView.

---

## 1. Current Implementation: AnchorViewerDialog.tsx

**File:** `src/tui/components/AnchorViewerDialog.tsx`
**Lines:** 274

### Component Structure
```
AnchorViewerDialog (React.FC)
├── Props: isVisible, anchorPoints, onClose, onGetTurnDetails
├── State: selectedIndex, showTurnDetails, turnDetails
├── Uses: Dialog wrapper, VirtualList, TurnContentModal
└── Input: useInputCompat with CRITICAL priority
```

### Input Handling Issue (Line 114-115)
```typescript
// Arrow keys - let VirtualList handle these
if (key.upArrow || key.downArrow || key.pageUp || key.pageDown || key.home || key.end) {
  return false;  // <-- PROBLEM: Allows keystrokes to leak!
}
```

### Emoji Constants to Replace (Lines 30-35)
```typescript
const ANCHOR_TYPE_ICONS: Record<AnchorType, string> = {
  ErrorResolution: '🔧',   // Replace with: [Error]
  TaskCompletion: '✅',    // Replace with: [Task]
  UserCheckpoint: '📍',    // Replace with: [Check]
  FeatureMilestone: '🏁',  // Replace with: [Mile]
};
```

---

## 2. Pattern to Follow: WatcherCreateView.tsx

**File:** `src/tui/components/WatcherCreateView.tsx`
**Pattern:** Full-screen view with absolute positioning

### Key Pattern (Lines 88-110)
```typescript
useInputCompat({
  id: 'watcher-create-view',
  priority: InputPriority.CRITICAL,
  description: 'Watcher create view input handler',
  isActive: true,
  handler: (input, key) => {
    // Handle all input
    // ...
    return true;  // <-- CORRECT: Consumes all input
  },
});
```

### Layout Pattern (Full-screen with absolute positioning)
```typescript
<Box
  position="absolute"
  top={0}
  left={0}
  width={terminalWidth}
  height={terminalHeight}
  flexDirection="column"
>
```

---

## 3. Split Pane Pattern: SplitSessionView.tsx

**File:** `src/tui/components/SplitSessionView.tsx`
**Pattern:** Left/right split with VirtualList in each pane

### Split Layout (approximate structure)
```typescript
<Box flexDirection="row" height="100%">
  {/* Left Pane: Session List */}
  <Box width="40%" borderStyle="single">
    <VirtualList items={sessions} ... />
  </Box>
  
  {/* Right Pane: Details */}
  <Box width="60%" borderStyle="single">
    <VirtualList items={turnLines} ... />
  </Box>
</Box>
```

---

## 4. useInputCompat Usage Across Codebase

### Components Using CRITICAL Priority
Found via AST: `useInputCompat($$$ARGS)`

| Component | Priority | Returns true? |
|-----------|----------|---------------|
| AnchorViewerDialog | CRITICAL | ❌ NO (returns false for arrow keys) |
| AttachmentDialog | CRITICAL | ✅ YES |
| BoardView | CRITICAL | ✅ YES |
| WatcherCreateView | CRITICAL | ✅ YES |
| CheckpointViewer | CRITICAL | ✅ YES |

### Pattern Confirmed
All full-screen/modal views with CRITICAL priority return `true` to consume all input.
AnchorViewerDialog is the outlier causing keystroke leaks.

---

## 5. Files to Modify/Delete

### Create New
- `src/tui/components/AnchorView.tsx` - Full-screen view

### Delete
- `src/tui/components/AnchorViewerDialog.tsx` - After migration

### Modify
- `src/tui/components/AgentView.tsx` - Remove dialog state, render AnchorView conditionally

---

## 6. AnchorPoint Data Structure

**From:** `src/tui/types/anchor.ts`

```typescript
interface AnchorPoint {
  anchorType: AnchorType;  // ErrorResolution | TaskCompletion | UserCheckpoint | FeatureMilestone
  turnIndex: number;
  timestamp: number;
  weight: number;          // Confidence score (0.0 - 1.0)
  description: string;
}
```

### Metadata to Display in Left Pane
- Anchor type label (text, no emoji)
- Turn number
- Confidence score (weight)
- Relative timestamp ("2 min ago")
- Description (truncated)

---

## 7. VirtualList Integration

**File:** `src/tui/components/VirtualList.tsx`

### Props Needed for AnchorView
```typescript
<VirtualList<AnchorPoint>
  items={anchorPoints}
  renderItem={renderAnchorItem}
  keyExtractor={(item, idx) => `${item.turnIndex}-${idx}`}
  showScrollbar={true}
  isFocused={leftPaneFocused}
  onFocus={handleAnchorFocus}  // Updates right pane
/>
```

### For Right Pane (Turn Content)
```typescript
<VirtualList<string>
  items={turnContentLines}
  renderItem={renderLine}
  showScrollbar={true}
  selectionMode="scroll"  // No item selection, just scrolling
/>
```

---

## Conclusion

The refactor requires:
1. Create `AnchorView.tsx` following `WatcherCreateView` full-screen pattern
2. Use split-pane layout from `SplitSessionView`
3. Fix input handling: return `true` for ALL input at CRITICAL priority
4. Replace emoji icons with text labels
5. Display rich metadata in left pane
6. Remove `AnchorViewerDialog.tsx` and all references
