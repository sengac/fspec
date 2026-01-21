# AST Research: Watcher Input Message Format

## Research Goal
Understand existing patterns for StreamChunk constructors and BackgroundSession methods to implement WatcherInput.

## Search: StreamChunk constructors in types.rs
```
fspec research --tool=ast --pattern="pub fn user_input" --lang=rust --path=codelet/napi/src/types.rs
```

Result:
```
codelet/napi/src/types.rs:318:5:pub fn user_input(text: String) -> Self {
```

## Key Findings

### 1. StreamChunk Constructor Pattern
The `user_input` constructor at line 318 shows the pattern to follow:
- Takes a String parameter for text content
- Returns `Self` (StreamChunk)
- Sets `chunk_type` to a specific type name ("UserInput")
- Stores content in the `text` field

### 2. Existing Chunk Types (from types.rs analysis)
- `Text` - regular text output
- `Thinking` - reasoning content
- `ToolCall` - tool invocation
- `ToolResult` - tool execution result
- `ToolProgress` - streaming tool output
- `Done` - turn complete marker
- `UserInput` - user input message

### 3. Pattern to Follow for WatcherInput
Add `StreamChunk::watcher_input()` following the same pattern as `user_input()`:
```rust
pub fn watcher_input(formatted_message: String) -> Self {
    Self {
        chunk_type: "WatcherInput".to_string(),
        text: Some(formatted_message),
        // all other fields None
    }
}
```

### 4. BackgroundSession Method Pattern
From session_manager.rs, methods like `set_role()` and `get_role()` show the pattern:
- Take `&self` for read-only operations
- Use RwLock for thread-safe access
- Return Result for fallible operations

### 5. Message Format
The structured prefix format:
```
[WATCHER: {role_name} | Authority: {authority} | Session: {session_id}] {message}
```

## Architecture Decision
- WatcherInput struct: near SessionRole types (lines 74-92 in session_manager.rs)
- format_watcher_input() function: standalone function
- mpsc channel: added to BackgroundSession for async queuing
- StreamChunk::watcher_input(): in types.rs impl block
