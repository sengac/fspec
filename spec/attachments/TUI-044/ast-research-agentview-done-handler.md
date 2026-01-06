# AST Research: AgentView Done Handler Analysis

## Purpose
Identify the exact location in AgentView.tsx where the `chunk.type === 'Done'` handler marks streaming as complete, which is where markdown table formatting should be applied.

## Research Commands

```bash
# Find the Done handler
fspec research --tool=ast --pattern="chunk.type === 'Done'" --lang=tsx --path=src/tui/components/AgentView.tsx
```

## Results

### Done Handler Location
- File: `src/tui/components/AgentView.tsx`
- Line: 2219
- Column: 22

### Code Context (lines 2219-2243)
The `chunk.type === 'Done'` handler:
1. Removes empty streaming assistant messages at the end
2. Finds the last assistant message with `isStreaming: true`
3. Sets `isStreaming: false` on that message

This is the ideal location to apply markdown table formatting - after the message is complete but before it's marked as non-streaming.

## Implementation Point
The transformation should be applied at line ~2237-2239 where the message is being updated:
```typescript
updated[lastAssistantIdx] = {
  ...updated[lastAssistantIdx],
  content: formatMarkdownTables(updated[lastAssistantIdx].content), // ADD THIS
  isStreaming: false,
};
```

## Related State
- `setConversation` is used 77 times in the file
- `isStreaming` property on ConversationMessage tracks streaming state
- Line 593: `const [conversation, setConversation] = useState<ConversationMessage[]>([]);`
