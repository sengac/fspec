# BUG-110: Codex list_dir facade missing offset and limit pagination params

## Problem

The `CodexListDirFacade` only maps `dir_path`. The schema exposes `depth` but it is not mapped either. The Codex CLI spec defines `offset` and `limit` as pagination controls that are completely absent.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "list_dir"
params:
  - dir_path: String (required) - "Absolute path to the directory to list."
  - offset: Number - "The entry number to start listing from. Must be 1 or greater."
  - limit: Number - "The maximum number of entries to return."
  - depth: Number - "The maximum directory depth to traverse. Must be 1 or greater."
```

## Current Implementation

**File**: `codelet/tools/src/facade/codex.rs` (lines 188-226)

Schema includes `dir_path` and `depth` but not `offset` or `limit`:

```rust
fn definition(&self) -> ToolDefinition {
    ToolDefinition {
        name: "list_dir".to_string(),
        parameters: json!({
            "properties": {
                "dir_path": { ... },
                "depth": { ... }
            },
            "required": ["dir_path"],
        }),
    }
}
```

And `map_params()` only maps `dir_path`:

```rust
fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError> {
    let dir_path = extract_optional_string(&input, "dir_path");
    Ok(InternalLsParams::List { path: dir_path })
}
```

## Impact

- `depth` is in the schema but silently ignored by `map_params()`
- `offset` and `limit` are not in the schema at all
- Large directories will return all entries instead of paginated results
- The model cannot paginate through directory listings

## Recommended Fix

1. Add `offset` and `limit` to the schema
2. Map `depth`, `offset`, and `limit` through to the internal `LsTool` implementation
3. If `InternalLsParams` doesn't support these fields, extend it or implement pagination in the facade wrapper

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Facade file: `codelet/tools/src/facade/codex.rs:188-226`
- LsTool implementation: `codelet/tools/src/ls.rs`
