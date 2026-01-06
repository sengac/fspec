# AST Research: AgentView.tsx Structure for TUI-043

## Overview
This document captures AST research findings for implementing the /expand command feature.

## Key Code Locations

### 1. ConversationMessage Interface (line 288)
```
grep: interface ConversationMessage {
```
- Needs `fullContent?: string` field added for storing uncollapsed content

### 2. Turn Selection State (line 595)
```
ast: isTurnSelectMode at src/tui/components/AgentView.tsx:595
```
- `useState<boolean>(false)` - toggles via /select command
- New `expandedMessageIndices` state should be added nearby

### 3. Format Functions for Collapsed Output
```
ast: const formatCollapsedOutput at src/tui/components/AgentView.tsx:350
ast: const formatDiffForDisplay at src/tui/components/AgentView.tsx:441
```
- Both contain "(ctrl+o to expand)" hint message to change
- Need to create non-collapsed versions for expanded state

### 4. /select Command Handler (line 1191-1209)
```
ast: isTurnSelectMode at src/tui/components/AgentView.tsx:1195
```
- Pattern to follow for /expand command handler
- Located in handleSubmit callback

### 5. VirtualList Integration (lines 4902-4946)
```
ast: isTurnSelectMode at src/tui/components/AgentView.tsx:4902 (selectionMode prop)
ast: isTurnSelectMode at src/tui/components/AgentView.tsx:4904 (getNextIndex prop)
ast: isTurnSelectMode at src/tui/components/AgentView.tsx:4946 (getIsSelected prop)
```
- Need to add selectionRef prop to expose selectedIndex to parent

### 6. Turn Selection Mode UI Indicator (line 4726)
```
ast: isTurnSelectMode at src/tui/components/AgentView.tsx:4726
```
- Shows [SELECT] indicator when mode is active

## useState Locations (partial list at lines 580-636)
- conversation state at line 585
- isTurnSelectMode at line 595
- New expandedMessageIndices should be added near line 596

## Files to Modify
1. `src/tui/components/AgentView.tsx` - Main implementation
2. `src/tui/components/VirtualList.tsx` - Add selectionRef prop

## Implementation Order
1. Add selectionRef prop to VirtualList.tsx
2. Add expandedMessageIndices state to AgentView.tsx
3. Add fullContent field to ConversationMessage interface
4. Update formatCollapsedOutput/formatDiffForDisplay hint messages
5. Create non-collapsed format functions
6. Modify tool result handlers to store fullContent
7. Update conversationLines computation
8. Add /expand command handler
9. Add cache invalidation logic
