# AST Research: AgentView Rendering Structure

## Purpose
Identify integration points for post-stream markdown rendering in AgentView.tsx.

## Key Findings

### 1. ConversationLine Interface (line 296)
```typescript
interface ConversationLine {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  messageIndex: number;
  isSeparator?: boolean; // TUI-042: Empty line used as turn separator
}
```

**Action Required:** Add `isStreaming?: boolean` field to propagate streaming state from parent message.

### 2. wrapMessageToLines Function (line 4047)
```
src/tui/components/AgentView.tsx:4047:3:const wrapMessageToLines = (
```
This function converts a ConversationMessage into ConversationLine[]. Currently does not propagate isStreaming.

**Action Required:** Pass `msg.isStreaming` through to each ConversationLine created.

### 3. VirtualList renderItem Callback (line 4880)
```
renderItem={(line, index, isSelected, selectedIndex) => {
```
This is where line.content is rendered as `<Text>` components.

**Action Required:** 
- Check if `line.role === 'assistant'` AND `line.isStreaming === false`
- If true, render content through ink-markdown component instead of plain Text

### 4. ConversationMessage Interface (line 706)
```typescript
interface ConversationMessage {
  role: 'user' | 'assistant' | 'tool';
  content: string;
  isStreaming?: boolean;  // Already exists
  ...
}
```
The `isStreaming` field already exists on ConversationMessage - just needs to be propagated to ConversationLine.

## Integration Flow
1. `ConversationMessage.isStreaming` is set to `true` during streaming
2. On `Done` event, it's set to `false` (line 2239)
3. `wrapMessageToLines()` creates `ConversationLine[]` - needs to copy `isStreaming`
4. `renderItem()` renders each line - needs to detect `isStreaming === false` for markdown

## No Buffering Required
Content accumulates in `conversation` state via `setConversation()` updates during streaming.
When `isStreaming` becomes false, the full content is already in `msg.content`.
