# Plan: Fixed-Height Tool Output Preview in AgentModal

## Current Behavior
Currently, bash/shell tool output streams directly into the VirtualList conversation:
- Each output chunk appends to a tool message
- All output is preserved and scrollable
- Can result in very long messages that dominate the conversation view

## Proposed Solution: Fixed-Height Preview Component

### 1. Create a `ToolOutputPreview` Component

**Features:**
- Fixed height (e.g., 10 lines)
- Maintains a rolling buffer of the most recent output lines
- Shows overflow indicators (e.g., "123 lines hidden")
- Bordered box to visually separate from conversation
- Real-time streaming updates

**Component Structure:**
```typescript
interface ToolOutputPreviewProps {
  toolName: string;
  height?: number; // Default: 10
  isStreaming: boolean;
  onOutput?: (lines: string[]) => void; // Callback for final output
}
```

### 2. Modify ConversationMessage Type

Add a new message variant for tool execution:
```typescript
interface ConversationMessage {
  role: 'user' | 'assistant' | 'tool' | 'tool-execution';
  content: string;
  isStreaming?: boolean;
  // For tool-execution messages only:
  toolExecution?: {
    toolCallId: string;
    toolName: string;
    status: 'running' | 'completed' | 'error';
    previewComponent?: React.ReactNode; // The preview component instance
    finalOutput?: string; // Truncated output after completion
  };
}
```

### 3. Update Stream Handling in AgentModal

**Current flow (lines 1600-1633):**
- `ToolProgress` chunks append directly to conversation
- Creates/updates tool messages with full output

**New flow:**
1. When `ToolCall` event arrives:
   - Add a planning message (current behavior)
   - Create a new `tool-execution` message with embedded preview component

2. When `ToolProgress` chunks arrive:
   - Route chunks to the preview component (not conversation)
   - Component maintains its own rolling buffer
   - Updates display in real-time within fixed bounds

3. When `ToolResult` arrives:
   - Mark preview as complete
   - Replace with final truncated output (current 500-char preview)
   - Remove the streaming preview component

### 4. VirtualList Integration

**Rendering tool-execution messages:**
```typescript
// In conversation line rendering
if (message.role === 'tool-execution' && message.toolExecution?.previewComponent) {
  return message.toolExecution.previewComponent;
} else {
  // Regular text rendering
}
```

### 5. Benefits

1. **Cleaner conversation view**: Tool output doesn't overwhelm the chat
2. **Better performance**: VirtualList doesn't need to handle thousands of bash output lines
3. **Consistent experience**: Always shows last N lines during execution
4. **Memory efficient**: Only keeps a rolling buffer, not entire output
5. **Visual clarity**: Bordered preview makes it clear this is transient output

### 6. Implementation Steps

1. Create `ToolOutputPreview` component with:
   - Circular buffer for lines
   - Overflow tracking
   - Streaming state management
   - Visual styling (border, colors)

2. Add `tool-execution` message type to conversation

3. Modify `ToolProgress` handler to:
   - Find associated preview component
   - Call component's `appendOutput` method
   - Don't modify conversation directly

4. Modify `ToolResult` handler to:
   - Stop preview streaming
   - Extract final output preview
   - Replace preview component with static text

5. Update VirtualList item renderer to handle embedded components

### 7. Example Visual

```
┌─────────────────────────────────────┐
│ [bash] (streaming...)               │
│ ├── Installing dependencies...      │
│ ├── npm install react               │
│ ├── npm install @types/react        │
│ ├── npm install vite                │
│ ├── Compiling TypeScript...         │
│ ├── src/index.ts → dist/index.js    │
│ ├── src/utils.ts → dist/utils.js    │
│ ├── Build completed successfully    │
│ ├──                                 │
│ └── ↓ 47 more lines                 │
└─────────────────────────────────────┘
```

After completion, this would be replaced with:
```
[Tool result preview]
-------
  Installing dependencies...
  ...
  Build completed successfully
-------
```

### 8. Considerations

- **Scrollback**: Users lose ability to see full tool output
  - Could add "Show full output" button
  - Or save to debug log file

- **Copy/Paste**: Preview component needs to support text selection
  - May need to render as selectable text

- **Persistence**: When resuming sessions, show static preview only
  - Don't recreate streaming components

- **Multiple tools**: Handle multiple simultaneous tool executions
  - Each gets its own preview component
  - Track by toolCallId