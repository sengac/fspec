# AST Research: Existing AstGrepRefactorTool Implementation

## Research Summary

Analyzed existing `astgrep_refactor.rs` to understand current structure before enhancement.

## Search Results

### Tool Implementation
```
Pattern: impl rig::tool::Tool for AstGrepRefactorTool
Location: codelet/tools/src/astgrep_refactor.rs:351:1
```

### Args Struct
```
Pattern: pub struct AstGrepRefactorArgs
Location: codelet/tools/src/astgrep_refactor.rs:313:1
```

## Current Structure (from source review)

### AstGrepRefactorArgs (lines 313-330)
```rust
pub struct AstGrepRefactorArgs {
    pub pattern: String,
    pub language: String,
    pub source_file: String,
    pub target_file: Option<String>,    // Extract mode
    pub replacement: Option<String>,     // Replace mode
}
```

### Enhancement Points

**New parameters to add:**
1. `transforms: Option<HashMap<String, Transform>>` - Transform definitions
2. `batch: Option<bool>` - Enable batch mode (default false)
3. `preview: Option<bool>` - Enable preview/dry-run mode (default false)

**New types to add:**
```rust
pub enum Transform {
    Substring { source: String, start_char: Option<i32>, end_char: Option<i32> },
    Replace { source: String, replace: String, by: String },
    Convert { source: String, to_case: CaseType, separated_by: Option<Vec<Separator>> },
}

pub enum CaseType {
    LowerCase, UpperCase, Capitalize, CamelCase, SnakeCase, KebabCase, PascalCase
}

pub enum Separator {
    CaseChange, Underscore, Dash, Dot, Slash, Space
}
```

### Key Integration Points

1. **execute() method** (line 37-273) - Main entry point, needs to:
   - Handle new `transforms`, `batch`, `preview` parameters
   - Validate transforms only used with replace mode
   - Validate batch only used with replace mode
   - Implement preview logic (return without writing)
   - Implement batch logic (allow multiple matches)

2. **definition() method** (line 358-378) - Tool description needs:
   - Comprehensive pattern syntax documentation
   - Transform syntax examples
   - Batch and preview mode documentation

3. **Match validation** (lines 155-177) - Currently requires exactly one match:
   - Keep for single mode
   - Skip for batch mode
   - Keep for preview mode (show all matches)

## Dependencies

- `ast_grep_language` crate - Already used for pattern matching
- `regex` crate - Needed for replace transform
- No additional crates needed for case conversion (implement inline)
