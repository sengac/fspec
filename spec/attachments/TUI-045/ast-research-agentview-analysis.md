# AST Research for TUI-045: Select Mode Enter Key Opens Full Turn Modal

## Research Date
2025-01-06

## Purpose
Analyze AgentView.tsx and VirtualList.tsx to identify:
1. Current state variables to modify/remove
2. Key handler locations to modify
3. VirtualList onSelect callback pattern
4. formatCollapsedOutput function location

---

## Research Results

### 1. State Variables - isTurnSelectMode (existing, keep)

**Pattern:** `isTurnSelectMode`
**File:** `src/tui/components/AgentView.tsx`

| Line | Context |
|------|---------|
| 604 | State declaration: `const [isTurnSelectMode, setIsTurnSelectMode] = useState(false)` |
| 1253 | Check in /expand handler (TO BE REMOVED) |
| 4044 | Tab key toggle: `const newMode = !isTurnSelectMode` |
| 4856 | UI indicator: `{isTurnSelectMode && ...}` |
| 4909 | Separator selection check |
| 5024 | VirtualList selectionMode: `selectionMode={isTurnSelectMode ? 'item' : 'scroll'}` |
| 5027 | VirtualList groupBy |
| 5028 | VirtualList groupPaddingBefore |

**Action:** Keep this state, add showTurnModal check in Esc handler

---

### 2. State Variables - expandedMessageIndices (TO BE REMOVED)

**Pattern:** `expandedMessageIndices`
**File:** `src/tui/components/AgentView.tsx`

| Line | Context |
|------|---------|
| 605 | State declaration: `const [expandedMessageIndices, setExpandedMessageIndices] = useState<Set<number>>(new Set())` |
| 4189 | Usage in conversationLines: `const isExpanded = expandedMessageIndices.has(msgIndex)` |
| 4226 | Dependency in useMemo: `[deferredConversation, terminalWidth, expandedMessageIndices]` |

**Action:** REMOVE this state entirely, along with all usages

---

### 3. formatCollapsedOutput Function

**Pattern:** `formatCollapsedOutput`
**File:** `src/tui/components/AgentView.tsx`

| Line | Context |
|------|---------|
| 352 | Function definition |
| 2137 | Usage in tool result processing |
| 2144 | Usage in tool result processing |
| 3185 | Usage in restored message processing |

**Action:** Update hint text from `(select turn to /expand)` to `(Enter to view full)` at line 352

---

### 4. Escape Key Handlers

**Pattern:** `key.escape`
**File:** `src/tui/components/AgentView.tsx`

| Line | Context |
|------|---------|
| 3629 | Search mode handler |
| 3672 | Resume mode handler |
| 3700 | Provider selector handler |
| 3727 | Model selector filter mode handler |
| 3754 | Model selector handler |
| 3897 | Settings filter mode handler |
| 3924 | Settings tab handler |
| **4028** | **MAIN Esc handler - TARGET FOR MODIFICATION** |

**Action:** Modify line 4028 handler to add showTurnModal and isTurnSelectMode priority checks BEFORE existing logic

---

### 5. VirtualList onSelect Callback

**Pattern:** `onSelect`
**File:** `src/tui/components/VirtualList.tsx`

| Line | Context |
|------|---------|
| 473 | Condition check: `else if (key.return && onSelect)` |
| 474 | Callback invocation: `onSelect(items[selectedIndex], selectedIndex)` |

**Action:** Add onSelect prop to main VirtualList in AgentView.tsx (line 4891) to open modal

---

### 6. Main VirtualList Location

**Pattern:** `VirtualList`
**File:** `src/tui/components/AgentView.tsx`

| Line | Context |
|------|---------|
| 28 | Import statement |
| 4891 | Main VirtualList JSX component |

**Action:** 
- Add `onSelect` prop at line ~4891
- Update `isFocused` prop to include `&& !showTurnModal`

---

## Summary of Changes Required

### Removals
1. **Line 605:** Remove `expandedMessageIndices` state
2. **Lines 1249-1281:** Remove `/expand` command handler
3. **Line 4189:** Remove expansion check in conversationLines
4. **Line 4226:** Remove expandedMessageIndices from useMemo deps

### Additions
1. **After line 605:** Add `showTurnModal` and `modalMessageIndex` state
2. **Line 4028:** Add modal and select mode checks in Esc handler
3. **Line ~4891:** Add `onSelect` callback to VirtualList
4. **After conversationLines useMemo:** Add `modalLines` useMemo
5. **Before closing Box:** Add modal JSX component

### Modifications
1. **Line 352:** Update formatCollapsedOutput hint text
2. **Line 5022:** Update isFocused to include `&& !showTurnModal`

---

## Code Patterns Verified

### VirtualList onSelect Pattern (from VirtualList.tsx)
```typescript
// Line 473-474
} else if (key.return && onSelect) {
  onSelect(items[selectedIndex], selectedIndex);
}
```

This confirms VirtualList calls onSelect on Enter key when in item selection mode.

### Current Esc Handler Pattern (from AgentView.tsx line 4028)
```typescript
if (key.escape) {
  if (isLoading && sessionRef.current) {
    sessionRef.current.interrupt();
  } else if (inputValue.trim() !== '') {
    setInputValue('');
  } else {
    onExit();
  }
  return;
}
```

This needs to be modified to add showTurnModal and isTurnSelectMode checks BEFORE existing checks.
