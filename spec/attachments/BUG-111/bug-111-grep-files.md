# BUG-111: Codex grep_files facade does not map include or limit params

## Problem

The `CodexGrepFilesFacade` exposes `include` and `limit` in its JSON schema but `map_params()` only extracts `pattern` and `path`. Both `include` and `limit` are silently ignored.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "grep_files"
params:
  - pattern: String (required) - "Regular expression pattern to search for."
  - include: String - "Optional glob that limits which files are searched (e.g. '*.rs')"
  - path: String - "Directory or file path to search. Defaults to session working directory."
  - limit: Number - "Maximum number of file paths to return (defaults to 100)."
```

## Current Implementation

**File**: `codelet/tools/src/facade/codex.rs` (lines 240-287)

Schema correctly includes all four params. But `map_params()` drops `include` and `limit`:

```rust
fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
    let pattern = extract_required_string(&input, "pattern", "grep_files")?;
    let path = extract_optional_string(&input, "path");
    Ok(InternalSearchParams::Grep { pattern, path })
}
```

The test `test_codex_grep_files_facade_with_include_filter` even acknowledges this with a comment:

```rust
// `include` is accepted in the schema but not mapped to InternalSearchParams
// (our internal grep handles file filtering separately)
```

## Impact

- When the model sends `{"pattern": "TODO", "include": "*.rs"}`, the glob filter is ignored and all files are searched
- When the model sends `{"limit": 10}`, more than 10 results may be returned
- This wastes context window tokens on unwanted results

## Recommended Fix

1. **include**: Map to the `GrepTool`'s file type filter mechanism. Check if `InternalSearchParams::Grep` supports a glob filter field — if not, add one. The internal ripgrep implementation likely supports `--glob` or `--type` flags.
2. **limit**: Map to a max results cap. Check if the internal grep implementation supports limiting output count.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Facade file: `codelet/tools/src/facade/codex.rs:240-287`
- GrepTool implementation: `codelet/tools/src/grep.rs`
