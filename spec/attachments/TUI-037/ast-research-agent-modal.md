# AST Research: AgentModal.tsx Tool Display

## Key Locations Found

### conversationLines memo (line 2914)
```
src/tui/components/AgentModal.tsx:2914:3:const conversationLines = useMemo((): ConversationLine[] => {
```
This is where conversation messages are converted to display lines.

### toolCall handling (lines 1394-1419)
```
src/tui/components/AgentModal.tsx:1394:17:toolCall
src/tui/components/AgentModal.tsx:1399:38:toolCall
src/tui/components/AgentModal.tsx:1401:27:toolCall
src/tui/components/AgentModal.tsx:1405:17:toolCall
src/tui/components/AgentModal.tsx:1406:19:toolCall
src/tui/components/AgentModal.tsx:1410:55:toolCall - Tool content formatting
src/tui/components/AgentModal.tsx:1418:22:toolCall
src/tui/components/AgentModal.tsx:1419:35:toolCall
```
Tool call handling and content formatting.

### toolResult handling (lines 2235-2238)
```
src/tui/components/AgentModal.tsx:2235:23:toolResult
src/tui/components/AgentModal.tsx:2236:21:toolResult
src/tui/components/AgentModal.tsx:2237:35:toolResult
src/tui/components/AgentModal.tsx:2238:37:toolResult
```
Tool result processing.

## Implementation Plan

1. **Tool Header Format** (line ~1410)
   - Change from `[Planning to use tool: ${toolCall.name}]` to `● ${toolCall.name}(${args})`

2. **conversationLines memo** (line ~2914)
   - Modify line rendering to use tree connectors (L character)
   - Add collapse logic with `... +N lines (ctrl+o to expand)`

3. **Streaming Window**
   - Implement 10-line rolling buffer for real-time output
   - New lines push older lines out of view

4. **Bug Fix: Persistent Header**
   - Ensure tool header messages are never removed from conversation
   - Investigate why headers currently disappear for Bash tools
