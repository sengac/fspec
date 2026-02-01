# StreamChunk Discriminated Union Refactoring Plan

## Problem Statement

The current `StreamChunk` type in `codelet/napi/src/types.rs` uses a **string-based discriminator pattern** that forces TypeScript to perform **fragile string parsing** to determine how to handle different chunk types.

### Current Architecture (Problematic)

```rust
// Rust: types.rs
#[napi(object)]
pub struct StreamChunk {
    #[napi(js_name = "type")]
    pub chunk_type: String,  // "Status", "Text", "Done", etc.
    pub text: Option<String>,
    pub status: Option<String>,  // Free-form string - NO STRUCTURE
    // ... many optional fields
}
```

```typescript
// TypeScript: AgentView.tsx (line 4336)
// FRAGILE: String parsing to filter compaction messages
if (!statusMessage.includes('compacted') && !statusMessage.includes('summary') &&
    !statusMessage.includes('compacting') && !statusMessage.includes('Compacting') &&
    statusMessage !== 'paused' && statusMessage !== 'running' && ...) {
```

### Observed Bug

When running `/compact`, a "compacting" message appears in the conversation area even though it should only appear in the input area. The root cause:

1. Rust sends `{ type: "Status", status: "compacting" }` for **session state changes**
2. Rust also sends `{ type: "Status", status: "Some user message" }` for **user notifications**
3. TypeScript cannot distinguish between them without string parsing
4. The filter logic is **incomplete and fragile** - any sentence containing "compacting" would be incorrectly filtered

## Research Findings

### napi-rs Discriminated Union Support

After examining `/tmp/napi-rs/examples/napi/src/enum.rs`, napi-rs provides **proper discriminated union support** using the `#[napi(discriminant = "type")]` attribute:

```rust
// From napi-rs examples/napi/src/enum.rs (lines 65-71)
#[napi(discriminant = "type2")]
pub enum StructuredKind {
    Hello,
    Greeting { name: String },
    Birthday { name: String, age: u8 },
    Tuple(u32, u32),
}
```

This generates proper TypeScript discriminated unions:

```typescript
// Generated TypeScript (from snapshot typegen.spec.ts.md)
export type StructuredKind =
  | { type2: 'Hello' }
  | { type2: 'Greeting', name: string }
  | { type2: 'Birthday', name: string, age: number }
  | { type2: 'Tuple', field0: number, field1: number }
```

## Proposed Solution

### New Rust Types

```rust
// codelet/napi/src/types.rs

/// Session state for internal state machine tracking
/// NOT for conversation display - use SessionStateChange chunk
#[napi(string_enum)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Idle,
    Running,
    Paused,
    Compacting,
    Interrupted,
}

/// Compaction progress details
#[napi(object)]
#[derive(Debug, Clone)]
pub struct CompactionProgressInfo {
    pub phase: String,
    pub current: u32,
    pub total: u32,
}

/// User notification severity levels
#[napi(string_enum)]
#[derive(Debug, Clone)]
pub enum NotificationSeverity {
    Info,
    Warning,
    Error,
}

/// Stream chunk - proper discriminated union
/// The type system enforces correct handling in TypeScript
#[napi(discriminant = "type")]
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Text content from assistant
    Text { content: String },
    
    /// Thinking/reasoning content (extended thinking)
    Thinking { content: String },
    
    /// Tool invocation
    ToolCall {
        id: String,
        name: String,
        input: String,
    },
    
    /// Tool execution result
    ToolResult {
        tool_call_id: String,
        content: String,
        is_error: bool,
    },
    
    /// Tool execution progress (streaming output)
    ToolProgress {
        tool_call_id: String,
        tool_name: String,
        output_chunk: String,
        is_stderr: bool,
    },
    
    /// Session state change - INTERNAL, not for conversation display
    /// TypeScript should update state machine, NOT add to conversation
    SessionStateChange {
        state: SessionState,
        /// Only present when state == Compacting
        compaction_progress: Option<CompactionProgressInfo>,
    },
    
    /// User-facing notification - DISPLAY in conversation
    UserNotification {
        message: String,
        severity: NotificationSeverity,
    },
    
    /// Token usage update
    TokenUpdate { tokens: TokenTracker },
    
    /// Context window fill update
    ContextFillUpdate { context_fill: ContextFillInfo },
    
    /// User input (for resume/attach replay)
    UserInput { content: String },
    
    /// Stream complete
    Done,
    
    /// Error occurred
    Error { message: String },
    
    /// Agent interrupted by user
    Interrupted,
    
    /// Watcher pending injection
    WatcherPendingInjection {
        urgent: bool,
        content: String,
    },
}
```

### Generated TypeScript (Automatic)

```typescript
export type SessionState = 'Idle' | 'Running' | 'Paused' | 'Compacting' | 'Interrupted'

export type NotificationSeverity = 'Info' | 'Warning' | 'Error'

export interface CompactionProgressInfo {
  phase: string
  current: number
  total: number
}

export type StreamChunk =
  | { type: 'Text', content: string }
  | { type: 'Thinking', content: string }
  | { type: 'ToolCall', id: string, name: string, input: string }
  | { type: 'ToolResult', toolCallId: string, content: string, isError: boolean }
  | { type: 'ToolProgress', toolCallId: string, toolName: string, outputChunk: string, isStderr: boolean }
  | { type: 'SessionStateChange', state: SessionState, compactionProgress?: CompactionProgressInfo }
  | { type: 'UserNotification', message: string, severity: NotificationSeverity }
  | { type: 'TokenUpdate', tokens: TokenTracker }
  | { type: 'ContextFillUpdate', contextFill: ContextFillInfo }
  | { type: 'UserInput', content: string }
  | { type: 'Done' }
  | { type: 'Error', message: string }
  | { type: 'Interrupted' }
  | { type: 'WatcherPendingInjection', urgent: boolean, content: string }
```

### TypeScript Handler (Clean, Type-Safe)

```typescript
// AgentView.tsx - handleStreamChunk becomes trivial
function handleStreamChunk(chunk: StreamChunk): void {
  switch (chunk.type) {
    case 'Text':
      appendToConversation({ type: 'assistant-text', content: chunk.content });
      break;
      
    case 'SessionStateChange':
      // INTERNAL: Update state machine, NOT conversation
      updateSessionState(chunk.state);
      if (chunk.state === 'Compacting' && chunk.compactionProgress) {
        setCompactionProgress(chunk.compactionProgress);
      }
      break;
      
    case 'UserNotification':
      // USER-FACING: Add to conversation
      addToConversation({ 
        type: 'status', 
        content: chunk.message,
        severity: chunk.severity 
      });
      break;
      
    case 'Done':
      markStreamComplete();
      break;
      
    // ... other cases with full type safety
  }
}
```

## Migration Strategy

### Phase 1: Add New Types (Backward Compatible)

1. Add new enum types alongside existing struct
2. Add conversion functions
3. Both old and new patterns work simultaneously

### Phase 2: Migrate Rust Producers

Update all call sites in Rust that produce StreamChunk:
- `session_manager.rs`: `handle_output()`, `set_status()`
- `output.rs`: NapiOutput emitter
- Tool executors

### Phase 3: Migrate TypeScript Consumers

Update AgentView.tsx and other consumers to use discriminated union pattern:
- Remove all string parsing logic
- Replace with exhaustive switch statements
- TypeScript compiler enforces all cases handled

### Phase 4: Remove Old Types

Delete deprecated struct-based StreamChunk and related string parsing code.

## Files Affected

### Rust (codelet/)

| File | Changes |
|------|---------|
| `napi/src/types.rs` | New StreamChunk enum, SessionState, NotificationSeverity |
| `napi/src/session_manager.rs` | Update `set_status()` to emit SessionStateChange |
| `napi/src/output.rs` | Update NapiOutput to emit proper variants |
| `core/src/tools/*.rs` | Update tool executors to emit proper variants |

### TypeScript (src/tui/)

| File | Changes |
|------|---------|
| `components/AgentView.tsx` | Remove string parsing, use switch on chunk.type |
| `hooks/useRustSessionState.ts` | May be simplified or removed (state comes via chunks now) |
| `__tests__/AgentView.test.tsx` | Update mocks to use new chunk types |

## Benefits

1. **Type Safety**: TypeScript compiler enforces exhaustive handling
2. **No String Parsing**: Zero runtime string matching
3. **Self-Documenting**: Type tells you exactly what data is available
4. **Impossible States**: Can't accidentally display internal state changes
5. **Refactoring Safety**: Renaming variants causes compile errors at all use sites
6. **IDE Support**: Full autocomplete and type inference

## Risks

1. **Breaking Change**: All consumers must be updated simultaneously
2. **NAPI Build**: Need to verify napi-rs builds discriminated unions correctly
3. **Serialization**: Need to verify JSON serialization matches expectations

## Acceptance Criteria

1. StreamChunk uses `#[napi(discriminant = "type")]` enum pattern
2. SessionStateChange is separate from UserNotification
3. No string parsing in TypeScript handlers
4. All existing tests pass after migration
5. Compaction status appears ONLY in input area, never in conversation
6. TypeScript handler uses exhaustive switch statement
