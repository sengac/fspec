# AST Research: StreamChunk Patterns for FspecTool Integration

## Research Purpose
Analyze existing StreamChunk patterns and FspecTool implementation to understand how to add structured FspecCommandRequest and FspecCommandResult variants.

## 1. Existing NAPI Types (codelet/napi/src/types.rs)

### Struct Patterns Found
```
codelet/napi/src/types.rs:10:1:pub struct CompactionProgress
codelet/napi/src/types.rs:32:1:pub struct NapiAnchorPoint
codelet/napi/src/types.rs:50:1:pub struct NapiToolCall
codelet/napi/src/types.rs:62:1:pub struct NapiFileModification
codelet/napi/src/types.rs:74:1:pub struct NapiTurnDetails
codelet/napi/src/types.rs:94:1:pub struct TokenTracker
codelet/napi/src/types.rs:125:1:pub struct DebugCommandResult
codelet/napi/src/types.rs:137:1:pub struct ToolCallInfo
codelet/napi/src/types.rs:146:1:pub struct ToolResultInfo
codelet/napi/src/types.rs:156:1:pub struct ToolProgressInfo
codelet/napi/src/types.rs:171:1:pub struct ContextFillInfo
codelet/napi/src/types.rs:186:1:pub struct WatcherPendingInjectionInfo
codelet/napi/src/types.rs:470:1:pub struct NapiProviderConfig
codelet/napi/src/types.rs:495:1:pub struct Message
codelet/napi/src/types.rs:504:1:pub struct CompactionResult
```

### Key Pattern: CompactionResult (line 504)
This is the model to follow for FspecCommandResult:
- Uses `#[napi(object)]` attribute
- Has typed fields with documentation
- Returns structured data from Rust to TypeScript

## 2. FspecToolFacadeWrapper (codelet/tools/src/facade/wrapper.rs)

### Location
```
codelet/tools/src/facade/wrapper.rs:340:1:pub struct FspecToolFacadeWrapper
codelet/tools/src/facade/wrapper.rs:359:1:impl Tool for FspecToolFacadeWrapper
```

### Current Implementation (lines 381-396)
Returns `FSPEC_INTERCEPT` string that needs to be replaced with structured request.

## 3. TypeScript Chunk Handling (src/tui/components/AgentView.tsx)

### CompactionComplete Pattern
```
src/tui/components/AgentView.tsx:3069:22:chunk.type === 'CompactionComplete'
src/tui/components/AgentView.tsx:4262:16:chunk.type === 'CompactionComplete'
```

This shows how TypeScript handles structured chunk types - direct field access without parsing.

## 4. Existing callFspecCommand Function

### Location
```
src/utils/fspec-init.ts:11:10:callFspecCommand (import)
src/utils/fspec-init.ts:207:3:callFspecCommand (usage)
codelet/napi/src/fspec.rs:24:1:pub fn call_fspec_command (implementation)
```

This is the existing JS callback mechanism that will be used to execute fspec commands.

## 5. Key Implementation Insights

### New Types Needed (types.rs)
```rust
/// Fspec command request from tool call
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecCommandRequest {
    pub command: String,
    pub args_json: String,
    pub project_root: String,
    pub tool_call_id: String,
}

/// Fspec command result after execution
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FspecCommandResult {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub system_reminder: Option<String>,
    pub tool_call_id: String,
}
```

### New StreamChunk Variants
Add to the enum (around line 224-335):
```rust
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
```

### TypeScript Handler Pattern
Follow the CompactionComplete pattern in AgentView.tsx:
```typescript
} else if (chunk.type === 'FspecCommandRequest') {
    const request = chunk.fspecRequest;
    // Execute via callFspecCommand callback
    // Return result via session_send_fspec_result
}
```

## 6. Files to Modify

| File | Changes |
|------|---------|
| `codelet/napi/src/types.rs` | Add FspecCommandRequest, FspecCommandResult structs and StreamChunk variants |
| `codelet/napi/src/session_manager.rs` | Add session_send_fspec_result NAPI function |
| `codelet/tools/src/facade/wrapper.rs` | Change FspecToolFacadeWrapper to signal request instead of FSPEC_INTERCEPT |
| `codelet/cli/src/interactive/stream_handlers.rs` | Remove FSPEC_INTERCEPT handling after migration |
| `src/tui/components/AgentView.tsx` | Handle FspecCommandRequest chunk type |
| `src/utils/fspec-init.ts` | May need updates for callback integration |

## 7. Test File Locations

Tests should be added to:
- `codelet/napi/src/types.rs` - Unit tests for new types
- `src/__tests__/` - Integration tests for TypeScript handling
- `src/tui/__tests__/` - Tests for AgentView chunk handling
