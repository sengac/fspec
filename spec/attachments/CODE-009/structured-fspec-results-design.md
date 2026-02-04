# Structured FspecTool Results via StreamChunk Discriminated Union

## Problem Statement

The current FspecTool implementation returns **unstructured string messages** that require fragile string parsing, rather than using the proper **typed StreamChunk discriminated union** pattern that already exists for other tools.

### Current Broken Flow

1. **FspecToolFacadeWrapper::call()** (wrapper.rs:381-396) returns a string:
   ```rust
   Ok(format!(
       "FSPEC_INTERCEPT: Command: '{}', Args: '{}', Root: '{}', Provider: '{}'",
       internal_params.command,
       internal_params.args,
       internal_params.project_root,
       self.facade.provider()
   ))
   ```

2. **handle_tool_result()** (stream_handlers.rs:346-367) intercepts this string and tries to parse it:
   ```rust
   if result_text.contains("FSPEC_INTERCEPT:") {
       if let Some(actual_result) = handle_fspec_session_error(&result_text) {
           // Only list-work-units is implemented!
           output.emit_tool_result(&tool_result.id, &actual_result, false);
           return Ok(());
       }
   }
   ```

3. **handle_fspec_session_error()** uses regex-style field extraction:
   ```rust
   let command = extract_field_from_fspec_error(error_message, "Command:")?;
   let args = extract_field_from_fspec_error(error_message, "Args:")?;
   let root = extract_field_from_fspec_error(error_message, "Root:")?;
   ```

### Problems

1. **String-based interception** - Fragile, requires regex-style parsing
2. **Only `list-work-units` implemented** - All other commands fail
3. **No proper data structure** - TypeScript can't type-check the response
4. **Interception at wrong layer** - Happens after tool result, not during execution
5. **No system reminder support** - Workflow guidance is lost

## Solution: Follow StreamChunk Pattern

The codebase already has an excellent pattern for typed tool results: **StreamChunk discriminated union**.

### How StreamChunk Works (types.rs)

```rust
/// NAPI-010: Stream chunk - proper discriminated union
#[napi(discriminant = "type")]
pub enum StreamChunk {
    Text { text: String, ... },
    ToolCall { tool_call: ToolCallInfo, ... },
    ToolResult { tool_result: ToolResultInfo, ... },
    CompactionComplete { compaction_result: CompactionResult },
    // ... other variants
}
```

### TypeScript Consumption (AgentView.tsx)

```typescript
// CompactionComplete example - NO STRING PARSING!
} else if (chunk.type === 'CompactionComplete') {
    const result = chunk.compactionResult;
    setCompactionReduction(Math.round(result.compressionRatio));
    compactionRef.current.endCompaction();
}
```

## Proposed Design

### 1. New StreamChunk Variants

Add two new variants to `codelet/napi/src/types.rs`:

```rust
/// Fspec command request - sent when LLM invokes FspecTool
/// TypeScript intercepts this and executes the command via JS callback
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecCommandRequest {
    /// The fspec command (e.g., "create-story", "show-work-unit")
    pub command: String,
    /// Command arguments as JSON string
    pub args_json: String,
    /// Project root directory
    pub project_root: String,
    /// Tool call ID for correlation with response
    pub tool_call_id: String,
}

/// Fspec command result - sent by TypeScript after executing command
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecCommandResult {
    /// Whether the command succeeded
    pub success: bool,
    /// Command output (structured data as JSON or human-readable text)
    pub data: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// System reminder for workflow orchestration (to be injected into LLM context)
    pub system_reminder: Option<String>,
    /// Tool call ID for correlation
    pub tool_call_id: String,
}

// Add to StreamChunk enum:
#[napi(discriminant = "type")]
pub enum StreamChunk {
    // ... existing variants ...
    
    /// Fspec command request - TypeScript must intercept and execute
    FspecCommandRequest {
        #[napi(js_name = "fspecRequest")]
        fspec_request: FspecCommandRequest,
    },
    
    /// Fspec command result - after TypeScript executes command
    FspecCommandResult {
        #[napi(js_name = "fspecResult")]
        fspec_result: FspecCommandResult,
    },
}
```

### 2. TypeScript Handler (AgentView.tsx)

```typescript
} else if (chunk.type === 'FspecCommandRequest') {
    const request = chunk.fspecRequest;
    
    // Execute fspec command via JS callback (already have callFspecCommand)
    try {
        const result = callFspecCommand(
            request.command,
            request.argsJson,
            request.projectRoot,
            fspecCallback  // The JS callback that executes commands
        );
        
        // Parse result and send back to Rust
        const parsed = JSON.parse(result);
        sendFspecResult({
            success: true,
            data: parsed.data,
            systemReminder: parsed.systemReminder,
            toolCallId: request.toolCallId,
        });
        
        // Inject system reminder into conversation if present
        if (parsed.systemReminder) {
            // Add to LLM context for workflow guidance
            injectSystemReminder(parsed.systemReminder);
        }
    } catch (err) {
        sendFspecResult({
            success: false,
            error: err.message,
            toolCallId: request.toolCallId,
        });
    }
}
```

### 3. New NAPI Functions

Add to `codelet/napi/src/session_manager.rs`:

```rust
/// Send fspec command result back to the session
/// Called by TypeScript after executing the fspec command
#[napi]
pub fn session_send_fspec_result(
    session_id: String, 
    result_json: String
) -> napi::Result<()> {
    // Parse result and inject into session's tool result flow
    // This unblocks the waiting agent loop
}
```

### 4. Rust Tool Changes

Modify `FspecToolFacadeWrapper::call()` to emit the structured request:

```rust
async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    let internal_params = self.facade.map_params(args.0)?;
    
    // Instead of returning a string, emit a FspecCommandRequest event
    // The session layer will:
    // 1. Emit StreamChunk::FspecCommandRequest to TypeScript
    // 2. Wait for session_send_fspec_result() callback
    // 3. Return the result to the agent
    
    // This requires the tool to have access to the session's broadcast channel
    // or use a different mechanism (oneshot channel, etc.)
    
    Err(ToolError::Custom(FspecPending {
        command: internal_params.command,
        args_json: internal_params.args,
        project_root: internal_params.project_root,
    }))
}
```

## Key Benefits

1. **Type Safety** - TypeScript can exhaustively switch on `chunk.type`
2. **Structured Data** - No string parsing, direct field access
3. **System Reminder Support** - Workflow guidance is properly typed and routable
4. **Extensibility** - Easy to add fields to the request/result structs
5. **Consistency** - Matches existing patterns (CompactionComplete, ToolResult, etc.)

## Implementation Steps

### Phase 1: Add Types (Small)
1. Add `FspecCommandRequest` and `FspecCommandResult` structs to types.rs
2. Add new `StreamChunk` variants
3. Regenerate TypeScript types via napi-rs

### Phase 2: Session Layer Handling
1. Add `session_send_fspec_result()` NAPI function
2. Add pending fspec request tracking to BackgroundSession
3. Modify agent loop to emit FspecCommandRequest and wait for result

### Phase 3: TypeScript Integration
1. Handle `FspecCommandRequest` in AgentView.tsx chunk processing
2. Execute via existing `callFspecCommand` mechanism
3. Call `session_send_fspec_result()` with structured result

### Phase 4: Remove Old Code
1. Remove `FSPEC_INTERCEPT` string pattern from wrapper.rs
2. Remove `handle_fspec_session_error()` from stream_handlers.rs
3. Remove `extract_field_from_fspec_error()` helper

## Alternatives Considered

### Alternative 1: Async Callback in Tool
Pass a callback channel to the tool at creation time, let it directly call into JS.

**Rejected because**: Tools are stateless and don't have session context. Would require significant architecture changes.

### Alternative 2: Synchronous File-Based IPC
Write request to temp file, poll for response file.

**Rejected because**: Fragile, slow, doesn't leverage existing StreamChunk infrastructure.

### Alternative 3: Keep String Parsing but Improve It
Just implement all commands in `execute_fspec_command_sync()`.

**Rejected because**: Defeats the purpose of NAPI integration. We have 100+ commands, and the sync implementation would duplicate all TypeScript logic in Rust.

## Relationship to CODE-002

This work unit (CODE-009) is a **prerequisite** for completing CODE-002 (Native Fspec Tool Integration).

CODE-002's goal is to call fspec TypeScript functions directly via NAPI-RS. The current `FSPEC_INTERCEPT` approach is a temporary hack. CODE-009 provides the proper infrastructure:

- CODE-009: Proper StreamChunk-based request/response flow
- CODE-002: Can then be marked complete once all commands work through this flow

## Files to Modify

### Rust
- `codelet/napi/src/types.rs` - Add new types and StreamChunk variants
- `codelet/napi/src/session_manager.rs` - Add `session_send_fspec_result()`
- `codelet/tools/src/facade/wrapper.rs` - Change FspecToolFacadeWrapper to emit request
- `codelet/cli/src/interactive/stream_handlers.rs` - Remove FSPEC_INTERCEPT handling

### TypeScript
- `codelet/napi/index.d.ts` - Auto-generated from Rust
- `src/tui/components/AgentView.tsx` - Handle FspecCommandRequest chunk

## Testing Strategy

1. **Unit Tests**: Test FspecCommandRequest/Result serialization in types.rs
2. **Integration Tests**: Test round-trip request → JS execution → result
3. **E2E Tests**: Verify fspec commands work in actual agent session

## Open Questions

1. **Blocking vs Async**: Should the agent loop block waiting for fspec result, or continue processing other events?
   - Recommendation: Block (with timeout), since fspec commands are synchronous from LLM's perspective

2. **Error Handling**: What happens if TypeScript crashes during fspec execution?
   - Recommendation: Timeout after 30s, return error result

3. **System Reminder Injection**: Where exactly should system reminders go?
   - Option A: Inject into messages array
   - Option B: Add to conversation context
   - Recommendation: Follow existing pattern for how bootstrap/help system reminders are handled
