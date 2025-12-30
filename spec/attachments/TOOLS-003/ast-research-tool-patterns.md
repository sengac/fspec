# AST Research: Codelet Tool Implementation Patterns

## Research Query
Pattern: `pub struct $NAME`
Language: Rust
Path: codelet/tools/src

## Key Findings

### Existing Tool Struct Patterns

The codelet tools follow a consistent pattern:

1. **Tool Struct** (unit struct or with config):
   - `AstGrepTool` (line 21 in astgrep.rs) - unit struct
   - `GrepTool` (line 48 in grep.rs) - unit struct
   - `EditTool` (line 15 in edit.rs) - unit struct
   - `WriteTool` (line 13 in write.rs) - unit struct
   - `ReadTool` (line 32 in read.rs) - unit struct
   - `GlobTool` (line 21 in glob.rs) - unit struct
   - `BashTool` (line 29 in bash.rs) - struct with shell field

2. **Args Struct** (derives Deserialize, Serialize, JsonSchema):
   - `AstGrepArgs` (line 321 in astgrep.rs)
   - `GrepArgs` (line 357 in grep.rs)
   - `EditArgs` (line 34 in edit.rs)
   - `WriteArgs` (line 32 in write.rs)
   - `ReadArgs` (line 98 in read.rs)
   - `GlobArgs` (line 48 in glob.rs)
   - `BashArgs` (line 202 in bash.rs)

### Implementation Template

Based on `AstGrepTool`, the new `AstGrepRefactorTool` should follow:

```rust
// File: codelet/tools/src/astgrep_refactor.rs

pub struct AstGrepRefactorTool;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct AstGrepRefactorArgs {
    /// AST pattern to match (ast-grep syntax)
    pub pattern: String,
    /// Programming language
    pub language: String,
    /// Source file to refactor
    pub source_file: String,
    /// Target file for extraction (mutually exclusive with replacement)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_file: Option<String>,
    /// Replacement pattern (mutually exclusive with target_file)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

impl rig::tool::Tool for AstGrepRefactorTool {
    const NAME: &'static str = "astgrep_refactor";
    type Error = ToolError;
    type Args = AstGrepRefactorArgs;
    type Output = String;

    // ... implementation
}
```

### Key Files to Reference

1. **codelet/tools/src/astgrep.rs** - Primary reference for AST operations
2. **codelet/napi/src/astgrep.rs** - NAPI refactor implementation (algorithm reference)
3. **codelet/tools/src/lib.rs** - Tool exports and common types

### Export Pattern (lib.rs)

```rust
mod astgrep_refactor;
pub use astgrep_refactor::AstGrepRefactorTool;
```
