# AST Research: Block Notifications

## Summary
Research findings for BLOCK-006: Block Notifications feature.

## Key Components

### Tool Facade Wrappers (codelet/tools/src/facade/wrapper.rs)

1. **BashToolFacadeWrapper** (line 453)
   - Currently does NOT have session_id field
   - Blocks commands via `check_bash_command()` (line 506)
   - Returns blocked error to AI but does NOT emit notification to TUI

2. **FileToolFacadeWrapper** (line 194)
   - Currently does NOT have session_id field
   - Does NOT currently check stage permissions
   - Need to add stage permission check AND notification emission

3. **FspecToolFacadeWrapper** (line 354)
   - HAS session_id field (line 360) - THIS IS THE PATTERN TO FOLLOW
   - Uses `session_id: Uuid` stored at construction time (TOOL-012 pattern)

4. **BridgeToolFacadeWrapper** (line 766)
   - HAS session_id field (line 771) - Another example of TOOL-012 pattern

### Notification Infrastructure (codelet/napi/src/types.rs)

1. **StreamChunk::UserNotification** (line 325-330)
   - Already exists with message and severity fields
   - NotificationSeverity enum: Info, Warning, Error

2. **StreamChunk::user_notification()** helper (line 454-456)
   - Factory function to create notification chunks

### Session Manager (codelet/napi/src/session_manager.rs)

1. **GLOBAL_CHUNK_CALLBACK** (line 37)
   - Global callback to emit chunks to TypeScript
   - Takes (session_id, chunk) pair

2. **SessionManager::instance().get_session()** (line 4237)
   - Can retrieve session by ID to call emit_chunk()

### TUI Handling (src/tui/components/AgentView.tsx)

1. **UserNotification handling** (line 673-680)
   - Creates `type: 'status'` messages in conversation
   - Already displays in conversation area

## Required Changes

### 1. Add session_id to BashToolFacadeWrapper
```rust
pub struct BashToolFacadeWrapper {
    facade: BoxedBashToolFacade,
    bash_tool: BashTool,
    session_id: Uuid,  // ADD THIS
}
```

### 2. Add session_id to FileToolFacadeWrapper
```rust
pub struct FileToolFacadeWrapper {
    facade: BoxedFileToolFacade,
    read_tool: ReadTool,
    write_tool: WriteTool,
    edit_tool: EditTool,
    session_id: Uuid,  // ADD THIS
}
```

### 3. Create notification emission helper
```rust
fn emit_block_notification(session_id: Uuid, action: &str, reason: &str) {
    if let Some(global_cb) = GLOBAL_CHUNK_CALLBACK.get() {
        let message = format!("AI was blocked from {} - {}", action, reason);
        let chunk = StreamChunk::user_notification(message, NotificationSeverity::Warning);
        global_cb.call(session_id.to_string(), chunk);
    }
}
```

### 4. Integrate stage permissions in FileToolFacadeWrapper
```rust
InternalFileParams::Write { file_path, content } => {
    // Check stage permissions
    let stage = get_current_work_unit_stage(self.session_id);
    if let Err(blocked) = check_write_permission(&file_path, stage.as_deref()) {
        emit_block_notification(self.session_id, &format!("writing {}", file_path), &blocked.reason);
        return Ok(FileOperationResult {
            success: false,
            content: None,
            error: Some(blocked.to_string()),
        });
    }
    // ... proceed with write
}
```

## Integration Points

1. **Session creation** - Need to pass session_id when creating tool wrappers
2. **Agent loop** - Where tools are instantiated with facades
3. **Notification display** - Already handled by AgentView
