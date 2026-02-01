# AST Research: StreamChunk Usage Analysis

## Research Query
Analyze StreamChunk type definition and all usage sites in Rust and TypeScript.

## Rust: StreamChunk Definition

**Location:** `codelet/napi/src/types.rs:219`
```
pub struct StreamChunk {
```

Current implementation uses a struct with `chunk_type: String` discriminator.

## Rust: StreamChunk::status() Calls

These are the sites that need to be migrated to SessionStateChange or UserNotification:

| File | Line | Context |
|------|------|---------|
| `codelet/napi/src/session_manager.rs` | 941 | `set_status()` - emits state changes (idle, running, paused, compacting, interrupted) |
| `codelet/napi/src/session_manager.rs` | 3817 | Stream event conversion |
| `codelet/napi/src/output.rs` | 226 | NapiOutput status emission |

## TypeScript: chunk.type Usage in AgentView.tsx

33 total usages of `chunk.type` found across the file:

### Early Stream Processing (lines 576-747)
- Lines 576, 583, 604, 631, 639, 674, 731, 747

### Buffered Output Processing (lines 1280-1286)
- Lines 1280, 1286

### Main Stream Chunk Handler (lines 2694-3277)
- Lines 2694, 2719, 2758, 2888, 3075, 3114, 3146, 3201, 3204, 3277
- **Line 3201**: Double check - uses chunk.type twice (likely nested condition)

### Stream Attach Handler (lines 4203-4411)
- Lines 4203, 4223, 4230, 4271, 4296, 4327, 4345, 4361, 4364, 4389, 4411
- **Line 4327**: This is where the Status chunk with string parsing exists

## Key Findings

1. **Single Definition**: StreamChunk is defined once in types.rs
2. **3 Status Emission Sites**: Three places in Rust emit Status chunks
3. **33 Handler Sites**: TypeScript has 33 places checking chunk.type
4. **Primary Status Handler**: Line 4327 contains the fragile string parsing logic

## Migration Impact

### Rust Changes Required
- `types.rs`: Convert struct to `#[napi(discriminant = "type")]` enum
- `session_manager.rs`: Update `set_status()` to emit SessionStateChange
- `output.rs`: Update NapiOutput to use proper variants

### TypeScript Changes Required
- All 33 `chunk.type` usages need to handle the new discriminated union
- Remove string parsing at line 4327 and replace with switch case
- TypeScript compiler will enforce exhaustive handling
