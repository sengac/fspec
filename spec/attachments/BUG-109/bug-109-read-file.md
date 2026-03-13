# BUG-109: Codex read_file facade missing mode and indentation params

## Problem

The `CodexReadFileFacade` only maps `file_path`, `offset`, and `limit`. The Codex CLI spec defines additional parameters for indentation-aware block reading that are missing from both the schema and the param mapping.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "read_file"
params:
  - file_path: String (required) - "Absolute path to the file"
  - offset: Number - "The line number to start reading from. Must be 1 or greater."
  - limit: Number - "The maximum number of lines to return."
  - mode: String - "Optional mode selector: 'slice' or 'indentation'"
  - indentation: Object - nested schema for indentation-aware block mode:
    - anchor_line: Number
    - max_levels: Number
    - include_siblings: Boolean
    - include_header: Boolean
    - max_lines: Number
```

## Current Implementation

**File**: `codelet/tools/src/facade/codex.rs` (lines 126-172)

Schema only has `file_path`, `offset`, `limit`. The `mode` and `indentation` parameters are completely absent.

```rust
fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
    let file_path = extract_required_string(&input, "file_path", "read_file")?;
    let offset = extract_optional_uint(&input, "offset");
    let limit = extract_optional_uint(&input, "limit");
    Ok(InternalFileParams::Read { file_path, offset, limit })
}
```

## Impact

When the model sends `mode: "indentation"` with an `indentation` block, those params are silently ignored and the file is read with default slice behavior. This degrades the model's ability to navigate code by semantic blocks.

## Recommended Fix

### Option A: Schema-only (minimal)
Add `mode` and `indentation` to the schema so the model doesn't get schema validation errors, but ignore them internally (document this limitation).

### Option B: Full implementation
1. Add `mode` and `indentation` to the schema
2. When `mode == "indentation"`:
   - Read the file
   - Use `anchor_line` to find the starting line
   - Return the indentation-scoped block (lines at anchor_line's indentation level and deeper, bounded by `max_levels` and `max_lines`)
   - Optionally include sibling blocks and file header
3. When `mode == "slice"` or absent: use existing `offset`/`limit` behavior

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Facade file: `codelet/tools/src/facade/codex.rs:126-172`
- ReadTool implementation: `codelet/tools/src/read.rs`
