# AST Research: Tool Implementation Patterns for CORE-004

## Overview

This document captures the AST analysis of existing tool implementations in codelet to guide the implementation of GrepTool and GlobTool.

## Tool Trait Definition (src/tools/mod.rs)

The `Tool` trait defines the contract all tools must implement:

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> &ToolParameters;
    async fn execute(&self, args: Value) -> Result<ToolOutput>;
}
```

Key data structures:
- `ToolParameters`: JSON Schema for tool parameters (type, properties, required)
- `ToolOutput`: Result type with content, truncated flag, and is_error flag

## Tool Registry Pattern (src/tools/mod.rs:107-114)

```rust
pub fn with_core_tools() -> Self {
    let mut registry = Self::new();
    registry.register(Box::new(BashTool::new()));
    registry.register(Box::new(ReadTool::new()));
    registry.register(Box::new(WriteTool::new()));
    registry.register(Box::new(EditTool::new()));
    registry
}
```

**Action**: GrepTool and GlobTool must be registered here.

## BashTool Pattern (src/tools/bash.rs)

### Constructor (lines 26-50)
- Creates `ToolParameters` with properties map
- Uses `json!()` macro for property definitions
- Sets required fields

### Execute Method (lines 87-158)
- Extracts args with `args.get("param").and_then(|v| v.as_str()).unwrap_or("")`
- Validates required parameters early, returns `ToolOutput::error()`
- Uses `process_output_lines()` for long line handling
- Uses `truncate_output()` and `format_truncation_warning()` from utils
- Returns `ToolOutput` with correct truncated and is_error flags

## ReadTool Pattern (src/tools/read.rs)

### Validation Pattern (lines 77-95)
```rust
let path = match require_absolute_path(file_path) {
    Ok(p) => p,
    Err(e) => return Ok(e),
};

if let Err(e) = require_file_exists(path, file_path) {
    return Ok(e);
}
```

Uses validation helpers from `src/tools/validation.rs`:
- `require_absolute_path()` - validates and returns Path
- `require_file_exists()` - checks file exists
- `require_is_file()` - checks path is a file
- `read_file_contents()` - reads file with error handling

### Output Formatting (lines 123-148)
- Formats lines with line numbers
- Uses `truncate_line_default()` for long lines
- Builds output with join and truncation warnings

## Truncation Utilities (src/tools/truncation.rs, src/utils/truncation.rs)

From `src/tools/limits.rs`:
```rust
pub struct OutputLimits;

impl OutputLimits {
    pub const MAX_OUTPUT_CHARS: usize = 30000;
    pub const MAX_LINE_LENGTH: usize = 2000;
    pub const MAX_LINES: usize = 2000;
}
```

Key functions:
- `truncate_output(&lines, max_chars)` - truncates output
- `format_truncation_warning(remaining, unit, char_truncated, max_chars)` - formats warning
- `truncate_line_default(line)` - replaces long lines with "[Omitted long line]"

## Implementation Requirements for GrepTool

Based on patterns:

1. **Struct definition**: Same pattern as BashTool/ReadTool
2. **Constructor**: Define parameters schema with:
   - `pattern` (required string)
   - `path` (optional string, default ".")
   - `output_mode` (optional: "files_with_matches", "content", "count")
   - `-i` (optional bool)
   - `multiline` (optional bool)
   - `-A`, `-B`, `-C` (optional integers)
   - `glob` (optional string)
   - `type` (optional string)

3. **Execute method**:
   - Parse all args using pattern above
   - Build regex matcher with case-insensitive and multiline options
   - Use `ignore::WalkBuilder` for gitignore-aware walking
   - Use `grep-searcher` for content searching
   - Format output based on output_mode
   - Apply truncation with existing utilities

## Implementation Requirements for GlobTool

1. **Parameters**:
   - `pattern` (required string)
   - `path` (optional string, default ".")

2. **Execute method**:
   - Use `ignore::WalkBuilder` for gitignore-aware walking
   - Use `globset::GlobBuilder` for pattern matching
   - Collect matching file paths
   - Sort by modification time (newest first)
   - Apply truncation

## Test File Locations

Existing tests in `tests/integration_test.rs` follow:
```rust
#[tokio::test]
async fn test_tool_registration() {
    let registry = ToolRegistry::with_core_tools();
    assert!(registry.get("Bash").is_some());
    // ...
}
```

GrepTool and GlobTool tests should follow same pattern.

## Dependencies to Add (Cargo.toml)

```toml
# Ripgrep core libraries
grep = "0.3"
grep-regex = "0.1"
grep-searcher = "0.1"

# Gitignore-aware file walking
ignore = "0.4"

# Glob pattern matching (part of ignore ecosystem)
globset = "0.4"
```
