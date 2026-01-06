# AST Research: AgentView Integration Points for /select Command

## Research Summary

This document captures AST research performed to identify integration points in AgentView.tsx for the /select command implementation.

## 1. handleSubmit Callback Location

**Pattern**: `const handleSubmit`
**Result**: 
```
src/tui/components/AgentView.tsx:1188:3:const handleSubmit = useCallback(async () => {
```

This is where command parsing happens. The /select command handler should be added here alongside /debug, /clear, /resume, /search.

## 2. useState Declarations

**Pattern**: `useState`
**Result**: Multiple useState calls found at lines 579-682

Key relevant state variables identified:
- Line 593: `isDebugEnabled` - Similar toggle state pattern to follow
- Lines 579-593: Core conversation and session state
- Lines 609-642: Model/provider selection state
- Lines 645-670: History, search, resume mode state

The new `isLineSelectMode` state should be added near `isDebugEnabled` (line 593) since it's a similar toggle pattern.

## 3. VirtualList Component Usage

**Pattern**: `<VirtualList`
**Result**:
```
src/tui/components/AgentView.tsx:4737:9:<VirtualList
```

This is the conversation VirtualList where `selectionMode` prop needs to be changed from hardcoded "scroll" to dynamic `{isLineSelectMode ? 'item' : 'scroll'}`.

## 4. Debug Indicator Pattern

**Pattern**: `isDebugEnabled`
**Result**:
```
src/tui/components/AgentView.tsx:593:10:isDebugEnabled   (state declaration)
src/tui/components/AgentView.tsx:4694:12:isDebugEnabled  (UI indicator)
```

The [SELECT] indicator should be added near the [DEBUG] indicator at line 4694, following the same pattern:
```tsx
{isLineSelectMode && (
  <Text color="cyan" bold>
    {' '}
    [SELECT]
  </Text>
)}
```

## Implementation Integration Points Summary

| What | Where | Line |
|------|-------|------|
| State declaration | After `isDebugEnabled` | ~593 |
| Command handler | In `handleSubmit` | ~1188+ |
| VirtualList prop | Conversation area | 4737 |
| UI indicator | Header bar | ~4694 |
| renderItem update | VirtualList renderItem | 4739+ |

## Files to Modify

1. **src/tui/components/AgentView.tsx** - All changes are in this single file
2. **src/tui/components/VirtualList.tsx** - NO CHANGES NEEDED (already supports both modes via TUI-032)

## Verification

All integration points have been identified via AST search. The implementation should:
1. Add state at line ~593
2. Add command handler at line ~1194 (after /debug handler)
3. Update VirtualList at line 4737 to use dynamic selectionMode
4. Add UI indicator at line ~4694 (after DEBUG indicator)
5. Update renderItem at line 4739 to use isSelected parameter
