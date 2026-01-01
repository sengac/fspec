# AST Research: Token Estimation Migration

## Summary
Research for PROV-002: Migrate from byte-based token estimation to tiktoken-rs.

## Existing Token Estimation Locations

### 1. interactive_helpers.rs (CLI)
**File**: `codelet/cli/src/interactive_helpers.rs`
**Lines**: 14-21
```rust
const APPROX_BYTES_PER_TOKEN: usize = 4;

fn estimate_tokens(text: &str) -> u64 {
    text.len().div_ceil(APPROX_BYTES_PER_TOKEN) as u64
}
```
**Used by**: Session loading for token estimation of persisted messages.

### 2. storage.rs (NAPI)
**File**: `codelet/napi/src/persistence/storage.rs`
**Lines**: 404-405
```rust
fn estimate_tokens(content: &str) -> u32 {
    // ~4 chars per token approximation
}
```
**Used by**: Message persistence for token count estimation.

### 3. compactor.rs (Core)
**File**: `codelet/core/src/compaction/compactor.rs`
**Line**: 162
```rust
// Estimate summary tokens (rough approximation: 1 token ≈ 4 characters)
```
**Used by**: Compaction decisions for context window management.

### 4. stream_loop.rs (CLI)
**File**: `codelet/cli/src/interactive/stream_loop.rs`
**Line**: 80
```rust
// Estimate tokens (~4 chars per token, rounded up)
```
**Used by**: Real-time token display during streaming.

## Read Tool Structure

**File**: `codelet/tools/src/read.rs`

### Key Types
- `ReadTool` - Main tool struct
- `ReadArgs` - Tool arguments (file_path, offset, limit)
- `ReadOutput` - Enum with Text and Image variants
- `FileType` - Enum for file type detection (Text, Image)

### Integration Points
- Token check should occur in `call()` method after file type detection
- For Text files, check token count after `String::from_utf8()` conversion
- Image files are exempt (handled by `FileType::Image` match arm)

## Error Handling

**File**: `codelet/tools/src/error.rs`

### Existing Error Variants
- `Timeout` - Operation timed out
- `Execution` - Command failed
- `File` - File I/O error
- `Validation` - Input validation failed
- `Pattern` - Regex/glob invalid
- `NotFound` - Resource not found
- `StringNotFound` - Edit tool specific
- `Language` - AstGrep language error

### Required Addition
Add new `TokenLimit` variant:
```rust
#[error("[{tool}] Token limit exceeded: {message}")]
TokenLimit { tool: &'static str, message: String },
```

## Core Crate Structure

**Location**: `codelet/core/src/`

### Existing Modules
- `compaction/` - Context compaction logic
- `compaction_hook.rs` - Compaction callbacks
- `lib.rs` - Public exports
- `rig_agent.rs` - LLM agent
- `token_usage.rs` - API token tracking
- `tool_specs.rs` - Tool specifications

### New Module
Add `token_estimator.rs` for shared TokenEstimator utility.

## Dependencies

### Current (codelet-core/Cargo.toml)
- tokio, futures, async-trait
- rig-core
- serde, serde_json, schemars
- anyhow, thiserror
- tracing
- uuid, chrono, regex

### Required Addition
```toml
tiktoken-rs = "0.6"
once_cell = "1.19"  # For lazy static initialization
```

## Integration Plan

1. Add tiktoken-rs to workspace Cargo.toml
2. Create TokenEstimator in codelet-core
3. Add TokenLimit error variant to codelet-tools
4. Update ReadTool to check token limits
5. Migrate all estimate_tokens() functions
