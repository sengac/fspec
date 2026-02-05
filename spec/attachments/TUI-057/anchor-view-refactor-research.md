# Research: Anchor View Refactor (Dialog → Full-Screen View)

## Problem Statement

The current anchor viewer (`/anchors` command) is implemented as a modal dialog (`AnchorViewerDialog.tsx`) that doesn't properly trap all input. Users report that some keystrokes "leak through" to underlying components, causing unexpected behavior.

## Current Implementation Analysis

### AnchorViewerDialog.tsx

**Location:** `src/tui/components/AnchorViewerDialog.tsx`

**Architecture:**
- Uses `Dialog` base component from `src/components/Dialog.tsx`
- Implements `useInputCompat` with `CRITICAL` priority
- Renders inside AgentView.tsx as an overlay
- Uses `VirtualList` for anchor display

**Input Handling Code (lines 94-121):**
```typescript
useInputCompat({
  id: 'anchor-viewer-dialog',
  priority: InputPriority.CRITICAL,
  description: 'Anchor viewer dialog navigation',
  isActive: isVisible && !showTurnDetails,
  handler: (input, key) => {
    if (key.return) {
      void handleViewDetails();
      return true;
    }
    // Type shortcuts (E, T, F, U)
    const lowerInput = input.toLowerCase();
    if (TYPE_SHORTCUTS[lowerInput]) {
      findAnchorByType(TYPE_SHORTCUTS[lowerInput]);
      return true;
    }
    // Arrow keys - let VirtualList handle these
    if (key.upArrow || key.downArrow || key.pageUp || key.pageDown || key.home || key.end) {
      return false;  // <-- BUG: Not consuming, letting through
    }
    // Consume all other input when dialog is visible
    return true;
  },
});
```

**Issues Identified:**
1. Arrow keys explicitly return `false`, not consuming input
2. VirtualList has its own input handling that may conflict
3. Dialog component overlays but doesn't fully isolate input context
4. `TurnContentModal` nested inside creates another input layer

### Comparison: WatcherCreateView.tsx (Full-Screen View Pattern)

**Location:** `src/tui/components/WatcherCreateView.tsx`

**Architecture:**
- Full-screen overlay using `position="absolute"` with terminal dimensions
- Takes over entire terminal space
- Uses `useInputCompat` with `CRITICAL` priority
- Returns `true` for ALL input at the end (complete consumption)

**Key Differences:**
```typescript
// WatcherCreateView - Full isolation
handler: (input, key) => {
  // ... handle specific keys ...
  return true; // Consume ALL input when form is active
},
```

vs

```typescript
// AnchorViewerDialog - Partial isolation
handler: (input, key) => {
  // ... handle specific keys ...
  if (key.upArrow || key.downArrow || ...) {
    return false;  // Let through to VirtualList
  }
  return true;
},
```

### Comparison: SplitSessionView.tsx (Watcher View)

**Location:** `src/tui/components/SplitSessionView.tsx`

**Features:**
- Full-screen split pane layout
- Manages multiple VirtualList instances
- Has selection mode per pane
- Handles cross-pane navigation
- Shows detailed header with model info, tokens, etc.

**Relevant Patterns:**
- Selection state: `useTurnSelection()` hook
- Pane switching: `activePane` state
- Turn content modal: `onOpenTurnContent` callback
- Keyboard hints in input placeholder

## Proposed Architecture: AnchorView

### Full-Screen View Benefits

1. **Complete Input Isolation** - No input leakage to underlying components
2. **More Screen Real Estate** - Can show anchor list + details side-by-side
3. **Better Navigation** - Can implement rich keyboard navigation without conflicts
4. **Editing Capability** - Space to add/edit/delete anchors
5. **Context Display** - Can show full turn content alongside anchors

### Suggested Layout

```
┌─────────────────────────────────────────────────────────────────────────┐
│ 📍 Anchor Points - Session: abc-123                     [4 anchors] ESC │
├─────────────────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────┐ ┌─────────────────────────────────────┐ │
│ │ ANCHORS                    │ │ TURN DETAILS                        │ │
│ │                            │ │                                     │ │
│ │ ▸ ✅ TaskCompletion  (0.91)│ │ Turn 14 - User:                     │ │
│ │     Turn 14 • 04:35        │ │ "Can you analyze the session..."    │ │
│ │                            │ │                                     │ │
│ │   🔧 ErrorResolution (0.85)│ │ Assistant:                          │ │
│ │     Turn 8 • 03:22         │ │ "I found the issue with the..."     │ │
│ │                            │ │                                     │ │
│ │   📍 UserCheckpoint  (0.80)│ │ Tool Calls:                         │ │
│ │     Turn 5 • 02:45         │ │ • Read: src/auth.ts                 │ │
│ │                            │ │ • Edit: src/auth.ts (lines 45-62)   │ │
│ │   🏁 FeatureMilestone(0.75)│ │ • Bash: npm test                    │ │
│ │     Turn 2 • 01:30         │ │                                     │ │
│ │                            │ │ Status: ✅ Success                   │ │
│ └─────────────────────────────┘ └─────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────────────┤
│ ↑↓ Navigate │ Enter View │ E Edit │ D Delete │ A Add │ E/T/F/U Jump    │
└─────────────────────────────────────────────────────────────────────────┘
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| ↑/↓ | Navigate anchor list |
| Enter | View full turn content modal |
| E | Edit selected anchor description |
| D | Delete selected anchor (with confirmation) |
| A | Add manual anchor at current position |
| Shift+E | Jump to first ErrorResolution |
| Shift+T | Jump to first TaskCompletion |
| Shift+F | Jump to first FeatureMilestone |
| Shift+U | Jump to first UserCheckpoint |
| ←/→ | Switch focus between panes |
| Tab | Toggle selection mode |
| Esc | Exit anchor view |

### New Capabilities

1. **Edit Anchor Description** - Modify the human-readable description
2. **Delete Anchor** - Remove an anchor point (useful for cleanup)
3. **Add Manual Anchor** - Create a UserCheckpoint at any turn
4. **View Full Content** - See complete turn context (user message, assistant response, tool calls, file changes)
5. **Filter by Type** - Show only specific anchor types

## Data Flow

### Current (Dialog)

```
AgentView
  └── /anchors command
        └── sessionGetAnchorPoints(sessionId)
              └── AnchorViewerDialog
                    └── VirtualList (anchors)
                          └── TurnContentModal (nested)
```

### Proposed (View)

```
AgentView
  └── /anchors command
        └── setShowAnchorView(true)

AnchorView (full screen, replaces AgentView content)
  ├── SessionHeader (reused)
  ├── Split Pane Layout
  │     ├── Left: Anchor List (VirtualList)
  │     └── Right: Turn Details (live preview)
  ├── ConversationInputArea (for commands within view)
  └── TurnContentModal (for full expand on Enter)
```

## NAPI Functions Required

### Existing (Already Implemented)

```typescript
// Get all anchor points for session
sessionGetAnchorPoints(sessionId: string): AnchorPoint[]

// Get turn details (partial implementation)
getAnchorTurnDetails(turnIndex: number): Promise<AnchorTurnDetails | null>
```

### New Functions Needed

```typescript
// Edit an anchor's description
sessionUpdateAnchorDescription(
  sessionId: string, 
  turnIndex: number, 
  newDescription: string
): void

// Delete an anchor
sessionDeleteAnchor(
  sessionId: string, 
  turnIndex: number
): void

// Add a manual anchor (UserCheckpoint)
sessionAddManualAnchor(
  sessionId: string,
  turnIndex: number,
  description: string
): AnchorPoint
```

## Files to Create/Modify

### New Files

1. `src/tui/components/AnchorView.tsx` - Main full-screen view component
2. `src/tui/hooks/useAnchorNavigation.ts` - Navigation and selection logic
3. `src/tui/components/AnchorTurnPreview.tsx` - Right pane turn details component

### Files to Modify

1. `src/tui/components/AgentView.tsx`
   - Add `showAnchorView` state (replaces `showAnchorViewer`)
   - Render `AnchorView` when active (full-screen, like watcher view)
   - Remove `AnchorViewerDialog` usage

2. `codelet/napi/src/session.rs`
   - Add `session_update_anchor_description`
   - Add `session_delete_anchor`
   - Add `session_add_manual_anchor`

3. `codelet/core/src/compaction/anchor.rs`
   - Add mutation methods to session anchor storage

### Files to Delete/Deprecate

1. `src/tui/components/AnchorViewerDialog.tsx` - Replace entirely

## Implementation Phases

### Phase 1: Basic View Conversion
- Create `AnchorView.tsx` with full-screen layout
- Port anchor list rendering from dialog
- Add proper input isolation
- Wire up to `/anchors` command

### Phase 2: Split Pane Layout
- Add turn details preview pane
- Implement pane switching
- Add live preview on selection

### Phase 3: Edit Capabilities
- Add NAPI functions for mutations
- Implement edit/delete/add UI
- Add confirmation dialogs

### Phase 4: Enhanced Navigation
- Type-based jump shortcuts
- Filter by anchor type
- Search within anchors

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Breaking existing `/anchors` functionality | Feature flag to switch between dialog/view |
| NAPI mutation complexity | Start with read-only view, add mutations later |
| Input conflicts | Use proven pattern from WatcherCreateView |
| Large refactor scope | Phase implementation, keep dialog as fallback |

## References

- TUI-056: Original anchor viewer story
- WATCH-009: Watcher creation dialog (form pattern)
- WATCH-010: Watcher split view (split pane pattern)
- INPUT-001: Centralized input handling system
