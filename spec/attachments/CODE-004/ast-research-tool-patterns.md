# AST Research: Tool Implementation Patterns

## Research Command Used
```bash
fspec research --tool=ast --pattern="async fn call" --lang=rust --path=codelet/tools/src/
```

## Existing Tool Implementations

Found Tool trait implementations in the following files:

### Core Tools
- `codelet/tools/src/grep.rs` - GrepTool implementation
- `codelet/tools/src/edit.rs` - EditTool implementation  
- `codelet/tools/src/fspec.rs` - FspecTool implementation (current)
- `codelet/tools/src/ls.rs` - LsTool implementation
- `codelet/tools/src/bash.rs` - BashTool implementation
- `codelet/tools/src/write.rs` - WriteTool implementation
- `codelet/tools/src/read.rs` - ReadTool implementation
- `codelet/tools/src/astgrep.rs` - AstGrepTool implementation
- `codelet/tools/src/web_search.rs` - WebSearchTool implementation
- `codelet/tools/src/glob.rs` - GlobTool implementation
- `codelet/tools/src/astgrep_refactor.rs` - AstGrepRefactorTool implementation

### Facade Wrappers
- `codelet/tools/src/facade/wrapper.rs` - Multiple facade wrapper implementations

## Key Findings

1. **Current FspecTool Status**: FspecTool already exists and implements the Tool trait at line 279 in `codelet/tools/src/fspec.rs`

2. **Implementation Pattern**: All tools follow the same pattern:
   - Implement `rig::tool::Tool` trait
   - Define `async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error>`
   - Have associated types: `Error = ToolError`, `Args = <ToolName>Args`, `Output = String`

3. **Critical Issue Identified**: Current FspecTool implementation uses `Command::new("fspec")` to spawn CLI processes, which contradicts the requirement for direct TypeScript function calls.

## Architecture Analysis

### Current Implementation Problem
The current FspecTool implementation in `execute_fspec_command()` method (around line 60) uses:
```rust
let mut cmd = Command::new("fspec");
cmd.args(&cmd_args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());
let output = cmd.output().await
```

This spawns the fspec CLI as a separate process, which:
- ❌ Defeats the purpose of direct TypeScript function calls
- ❌ Still has the 100-500ms process spawning overhead
- ❌ Does not achieve the performance goals

### Required Architecture Change
Need to replace CLI process spawning with direct calls to fspec TypeScript functions via the NAPI bindings established in CODE-003.

## Implementation Requirements

1. **Replace Command::new("fspec")** with direct function calls
2. **Use NAPI bindings** from CODE-003 to call TypeScript functions
3. **Maintain system reminder capture** functionality
4. **Preserve Tool trait interface** for compatibility
5. **Achieve 10-100x performance improvement** by eliminating process spawning

## Next Steps

1. Analyze how fspec TypeScript functions can be called directly
2. Modify `execute_fspec_command()` to use direct function calls
3. Ensure system reminders are still captured and returned
4. Test performance improvement vs CLI spawning