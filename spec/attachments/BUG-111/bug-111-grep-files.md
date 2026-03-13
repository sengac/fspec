# BUG-111: Codex grep_files facade does not map include or limit params

## Problem

The `CodexGrepFilesFacade` exposes `include` and `limit` in its JSON schema but `map_params()` only extracts `pattern` and `path`. Both `include` and `limit` are silently ignored.

**This is especially critical because `grep_files` with `include` is the Codex-native way to do glob-like file matching** — Codex has no standalone `glob` tool (see BUG-107). Without `include` working, the model has no way to filter searches by file type.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs` (verified 2026-03-13 from fresh clone):

```rust
fn create_grep_files_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        ("pattern", String, required, "Regular expression pattern to search for."),
        ("include", String, optional, "Optional glob that limits which files are searched (e.g. '*.rs' or '*.{ts,tsx}')."),
        ("path", String, optional, "Directory or file path to search. Defaults to session's working directory."),
        ("limit", Number, optional, "Maximum number of file paths to return (defaults to 100)."),
    ]);
}
```

## How Codex CLI Implements It

From `codex-rs/core/src/tools/handlers/grep_files.rs` (verified 2026-03-13):

```rust
#[derive(Deserialize)]
struct GrepFilesArgs {
    pattern: String,
    #[serde(default)]
    include: Option<String>,     // ← properly deserialized
    #[serde(default)]
    path: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,                // ← defaults to 100, max 2000
}

// In the handler:
let include = args.include.as_deref().map(str::trim).and_then(|val| {
    if val.is_empty() { None } else { Some(val.to_string()) }
});

let search_results = run_rg_search(
    pattern,
    include.as_deref(),  // ← passed through
    &search_path,
    limit,               // ← used to cap results
    &turn.cwd
).await?;

// In run_rg_search():
async fn run_rg_search(pattern, include, search_path, limit, cwd) {
    let mut command = Command::new("rg");
    command.arg("--files-with-matches")
           .arg("--sortr=modified")
           .arg("--regexp").arg(pattern);

    if let Some(glob) = include {
        command.arg("--glob").arg(glob);  // ← maps to rg --glob
    }

    // Results capped at limit (line 144):
    // Ok(parse_results(&output.stdout, limit))
}
```

Key behaviors:
- `include` is passed to `rg --glob` for file filtering
- `limit` caps the number of file paths returned (default 100, max 2000)
- Results are sorted by modification time (`--sortr=modified`)
- Default output mode is `--files-with-matches` (file paths only)

## Current Broken Implementation

**File**: `codelet/tools/src/facade/codex.rs`

Schema correctly includes all four params:
```rust
"include": { "type": "string", "description": "Optional glob..." },
"limit": { "type": "integer", "description": "Maximum number..." },
```

But `map_params()` drops them:
```rust
fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
    let pattern = extract_required_string(&input, "pattern", "grep_files")?;
    let path = extract_optional_string(&input, "path");
    Ok(InternalSearchParams::Grep { pattern, path })  // ← include and limit LOST
}
```

The `InternalSearchParams::Grep` enum variant only has `pattern` and `path`:
```rust
pub enum InternalSearchParams {
    Grep { pattern: String, path: Option<String> },
    Glob { pattern: String, path: Option<String> },
}
```

And the `SearchToolFacadeWrapper` constructs `GrepArgs` without include:
```rust
let grep_args = GrepArgs {
    pattern,
    path: resolved_path,
    output_mode: None,  // ← no include, no limit
};
```

The underlying `GrepTool::execute()` method DOES support glob filtering via a `"glob"` param (line 221-256 of `codelet/tools/src/grep.rs`):
```rust
let glob_pattern = args.get("glob").and_then(|v| v.as_str());
// ... builds globset filter and applies it to walker
```

But this is never reached because `include` is dropped before it gets there.

## Impact

- When the model sends `{"pattern": "TODO", "include": "*.rs"}`, all files are searched (not just .rs files)
- When the model sends `{"limit": 10}`, potentially hundreds of results are returned
- **The model's only way to do file-pattern matching is broken** — there is no standalone `glob` tool in native Codex
- Wastes context window tokens on unwanted results

## Required Fix

### 1. Extend `InternalSearchParams::Grep` to include new fields

```rust
pub enum InternalSearchParams {
    Grep {
        pattern: String,
        path: Option<String>,
        include: Option<String>,   // NEW: glob filter for file matching
        limit: Option<usize>,      // NEW: max results cap
    },
    Glob { ... },
}
```

### 2. Update `CodexGrepFilesFacade::map_params()` to extract include and limit

```rust
fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
    let pattern = extract_required_string(&input, "pattern", "grep_files")?;
    let path = extract_optional_string(&input, "path");
    let include = extract_optional_string(&input, "include");
    let limit = extract_optional_uint(&input, "limit");
    Ok(InternalSearchParams::Grep { pattern, path, include, limit })
}
```

### 3. Update `SearchToolFacadeWrapper` to pass include and limit through

In `wrapper.rs`, the `InternalSearchParams::Grep` match arm must pass `include` as `"glob"` to the GrepTool's execute args, and apply `limit` to cap results.

### 4. Update `GrepArgs` to support include/glob field

The `GrepArgs` struct in `grep.rs` needs an `include` or `glob` field so the `call()` method can pass it to `execute()`.

### 5. Update all other facades that use `InternalSearchParams::Grep`

Check ZAI and any other facades that construct `InternalSearchParams::Grep` — they'll need updating for the new fields (can use `None` defaults).

## Files to Change

1. `codelet/tools/src/facade/traits.rs` — Add `include` and `limit` to `InternalSearchParams::Grep`
2. `codelet/tools/src/facade/codex.rs` — Update `map_params()` to extract include/limit
3. `codelet/tools/src/facade/wrapper.rs` — Pass include/limit through to GrepTool
4. `codelet/tools/src/grep.rs` — Add `include` field to `GrepArgs`, wire through to `execute()`
5. `codelet/tools/src/facade/zai.rs` — Update ZAI grep facade for new enum fields
6. All other facades constructing `InternalSearchParams::Grep`

## References

- Codex CLI repo: https://github.com/openai/codex
- Tool spec: `codex-rs/core/src/tools/spec.rs` (create_grep_files_tool, line 1545)
- Handler: `codex-rs/core/src/tools/handlers/grep_files.rs` (GrepFilesArgs, run_rg_search)
- Our facade: `codelet/tools/src/facade/codex.rs:240-287`
- Our GrepTool: `codelet/tools/src/grep.rs` (already has glob support at line 221-256, just not wired)
- Our wrapper: `codelet/tools/src/facade/wrapper.rs:1116-1161`
- BUG-107: Removed non-native `glob` tool — makes this fix even more critical
