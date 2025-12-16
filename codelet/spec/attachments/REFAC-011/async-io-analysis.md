# Blocking I/O in Async Tool Implementations

## Summary

Several tool implementations use blocking `std::fs` operations inside async functions. While the Rust compiler allows this, it can block the async runtime's executor thread, reducing concurrency under load.

## Current State

### Example: ReadTool (`tools/src/read.rs`)

```rust
impl rig::tool::Tool for ReadTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // BLOCKING: std::fs::read_to_string blocks the async executor
        let content = read_file_contents(path)?;  // Uses std::fs internally
        // ...
    }
}
```

### Files with Blocking I/O

| File | Blocking Operations |
|------|---------------------|
| `tools/src/read.rs` | `std::fs::read_to_string` |
| `tools/src/write.rs` | `std::fs::write`, `std::fs::create_dir_all` |
| `tools/src/edit.rs` | `std::fs::read_to_string`, `std::fs::write` |
| `tools/src/glob.rs` | `std::fs::metadata`, directory walking |
| `tools/src/grep.rs` | File reading for content search |
| `tools/src/bash.rs` | `std::process::Command` (partially async via tokio) |
| `tools/src/astgrep.rs` | File reading for AST parsing |

### Why This Matters

1. **Single-threaded runtime**: If using `tokio::runtime::Builder::new_current_thread()`, blocking I/O halts ALL async tasks
2. **Multi-threaded runtime**: Blocking I/O occupies a worker thread, reducing parallelism
3. **Tool concurrency**: Multiple tool calls could run concurrently, but blocking I/O serializes them

### Why It's Low Priority for CLI

- CLI tools typically run single operations sequentially
- User waits for each operation anyway
- File I/O is fast on modern SSDs
- No significant user-facing impact observed

## Recommended Fix

### Option A: Use `tokio::fs` (Recommended)

```rust
use tokio::fs;

impl rig::tool::Tool for ReadTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // NON-BLOCKING: tokio::fs::read_to_string yields to executor
        let content = fs::read_to_string(&args.file_path).await
            .map_err(|e| ReadError::FileError(e.to_string()))?;
        // ...
    }
}
```

### Option B: Use `spawn_blocking` for CPU-intensive work

```rust
use tokio::task::spawn_blocking;

impl rig::tool::Tool for ReadTool {
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let path = args.file_path.clone();
        let content = spawn_blocking(move || {
            std::fs::read_to_string(&path)
        }).await
            .map_err(|e| ReadError::FileError(e.to_string()))?
            .map_err(|e| ReadError::FileError(e.to_string()))?;
        // ...
    }
}
```

### Comparison

| Approach | Pros | Cons |
|----------|------|------|
| `tokio::fs` | Clean, idiomatic | Adds tokio dependency to tools crate |
| `spawn_blocking` | Works with any sync code | Extra allocation, thread pool overhead |

## Implementation Steps

### Phase 1: Add tokio dependency to tools crate

```toml
# tools/Cargo.toml
[dependencies]
tokio = { version = "1", features = ["fs"] }
```

### Phase 2: Update validation helpers (`tools/src/validation.rs`)

```rust
// Before
pub fn read_file_contents(path: &Path) -> Result<String, ToolOutput> {
    std::fs::read_to_string(path).map_err(|e| ToolOutput::error(...))
}

// After
pub async fn read_file_contents(path: &Path) -> Result<String, ToolOutput> {
    tokio::fs::read_to_string(path).await.map_err(|e| ToolOutput::error(...))
}
```

### Phase 3: Update each tool's `call()` method

Add `.await` to file operations:

```rust
// Before
let content = read_file_contents(path)?;

// After
let content = read_file_contents(path).await?;
```

### Phase 4: Update tests

Tests may need `#[tokio::test]` if not already async.

## Files to Modify

| File | Changes |
|------|---------|
| `tools/Cargo.toml` | Add `tokio = { features = ["fs"] }` |
| `tools/src/validation.rs` | Make `read_file_contents`, `require_file_exists` async |
| `tools/src/read.rs` | Add `.await` to file reads |
| `tools/src/write.rs` | Use `tokio::fs::write`, `tokio::fs::create_dir_all` |
| `tools/src/edit.rs` | Add `.await` to file reads/writes |
| `tools/src/glob.rs` | Use `tokio::fs::metadata` or `spawn_blocking` for walkdir |
| `tools/src/grep.rs` | Add `.await` to file reads |
| `tools/src/astgrep.rs` | Add `.await` to file reads |

## Estimated Effort

- **Complexity**: Low-Medium
- **Risk**: Low (straightforward refactor)
- **Time**: 2-3 hours

## Verification

After implementation:

1. `cargo test` - All tests pass
2. `cargo clippy -- -D warnings` - No warnings
3. Manual testing - CLI operations work as expected
4. No observable performance regression

## Priority

**Low** - This is a code quality improvement, not a bug fix. The current implementation works correctly; it's just not optimal for async runtimes.
