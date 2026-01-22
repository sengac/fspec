# AST Research: Purple Watcher Input Display (WATCH-012)

## Purpose
Research existing code patterns for implementing watcher input display in purple/magenta color.

## Key Files to Modify

### 1. src/tui/types/conversation.ts
**Current State:**
- `MessageType` union: `'user-input' | 'assistant-text' | 'thinking' | 'tool-call' | 'status'`
- `ConversationLine.role`: `'user' | 'assistant' | 'tool'`

**Required Changes:**
- Add `'watcher-input'` to `MessageType` union
- Add `'watcher'` to `ConversationLine.role` union

### 2. src/tui/components/AgentView.tsx
**processChunksToConversation (line 440):**
```
src/tui/components/AgentView.tsx:440:1:const processChunksToConversation = (
```

**Color determination (line 6528):**
```typescript
const baseColor = line.role === 'user' ? 'green' : 'white';
```

**Required Changes:**
1. Add `parseWatcherPrefix()` function to extract role, authority, sessionId, content from watcher message prefix
2. Add `WatcherInput` chunk type handling in `processChunksToConversation`:
   - Parse prefix with `parseWatcherPrefix()`
   - Create `ConversationMessage` with `type: 'watcher-input'`
   - Format content as `👁️ RoleName> content`
3. Update color determination to handle watcher role:
   ```typescript
   const baseColor = line.role === 'user' ? 'green' : line.role === 'watcher' ? 'magenta' : 'white';
   ```

### 3. codelet/napi/src/types.rs (already implemented in WATCH-006)
**StreamChunk::watcher_input (line 365):**
```rust
pub fn watcher_input(formatted_message: String) -> Self {
    Self {
        chunk_type: "WatcherInput".to_string(),
        text: Some(formatted_message),
        ...
    }
}
```

## Watcher Message Format
From WATCH-006, the watcher message format is:
```
[WATCHER: role | Authority: level | Session: id]\ncontent
```

Example:
```
[WATCHER: Security Reviewer | Authority: Supervisor | Session: abc-123]
SQL injection vulnerability detected
```

## Regex Pattern for Parsing
```typescript
const WATCHER_PREFIX_REGEX = /^\[WATCHER: ([^|]+) \| Authority: (Supervisor|Peer) \| Session: ([^\]]+)\]\n/;
```

## Integration Points
1. `processChunksToConversation` - chunk processing
2. VirtualList render function - color application
3. ConversationLine flattening - role assignment

## Dependencies
- WATCH-006: WatcherInput StreamChunk type (already implemented)
