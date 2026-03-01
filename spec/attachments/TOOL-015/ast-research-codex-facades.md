# AST Research: Codex Tool Facade Implementation Patterns

## Research Date: 2026-03-01
## Work Unit: TOOL-015

## Facade Trait Implementations Found

### BashToolFacade implementations:
- `codelet/tools/src/facade/zai.rs:258` - `ZAIRunCommandFacade` (tool_name: "run_command")
- `codelet/tools/src/facade/bash.rs:16` - `GeminiRunShellCommandFacade` (tool_name: "run_shell_command")

### FileToolFacade implementations:
- `codelet/tools/src/facade/file_ops.rs:15` - `GeminiReadFileFacade`
- `codelet/tools/src/facade/file_ops.rs:81` - `GeminiWriteFileFacade`
- `codelet/tools/src/facade/file_ops.rs:139` - `GeminiReplaceFacade`
- `codelet/tools/src/facade/zai.rs:104` - `ZAIReadFileFacade` (tool_name: "read_file")
- `codelet/tools/src/facade/zai.rs:157` - `ZAIWriteFileFacade`
- `codelet/tools/src/facade/zai.rs:201` - `ZAIEditFileFacade`

### LsToolFacade implementations:
- `codelet/tools/src/facade/zai.rs:62` - `ZAIListDirFacade` (tool_name: "list_dir")
- `codelet/tools/src/facade/ls.rs:16` - `GeminiListDirectoryFacade`

### SearchToolFacade implementations:
- `codelet/tools/src/facade/zai.rs:300` - `ZAIGrepFilesFacade` (tool_name: "grep_files")
- `codelet/tools/src/facade/zai.rs:345` - `ZAIFindFilesFacade` (tool_name: "find_files")
- `codelet/tools/src/facade/search.rs:16` - `GeminiSearchFileContentFacade`
- `codelet/tools/src/facade/search.rs:71` - `GeminiGlobFacade`

## CodexProvider current tool registration (codex/mod.rs:277-289):
```rust
.tool(ReadTool::new(session_id))     // → replace with FileToolFacadeWrapper(CodexReadFileFacade)
.tool(WriteTool::new(session_id))    // → keep (no Codex equivalent)
.tool(EditTool::new(session_id))     // → keep (no Codex equivalent)
.tool(BashTool::new(session_id))     // → replace with BashToolFacadeWrapper(CodexShellCommandFacade)
.tool(GrepTool::new(session_id))     // → replace with SearchToolFacadeWrapper(CodexGrepFilesFacade)
.tool(GlobTool::new(session_id))     // → keep (no Codex equivalent)
.tool(LsTool::new(session_id))       // → replace with LsToolFacadeWrapper(CodexListDirFacade)
.tool(AstGrepTool::new(session_id))  // → keep
.tool(AstGrepRefactorTool::new(session_id)) // → keep
.tool(WebSearchTool::new(session_id)) // → keep
```

## Pattern from Z.AI provider (zai.rs:197-245):
Uses FacadeToolWrapper for all 7 tools, plus AstGrep, WebSearch, Fspec, Bridge.

## Key differences for Codex facades vs ZAI:
- `shell_command` (not `run_command`) - also has `workdir` param (maps to cwd on BashTool)
- `read_file` with `file_path` (same as ZAI)
- `list_dir` with `dir_path` (not `path`) - **key schema difference**
- `grep_files` with `include` param (glob filter, not in ZAI version)

## Files to create:
- `codelet/tools/src/facade/codex.rs` - 4 facade structs + tests

## Files to modify:
- `codelet/tools/src/facade/mod.rs` - add `mod codex;` and exports
- `codelet/providers/src/codex/mod.rs` - update create_rig_agent()
- `codelet/tools/src/facade/fspec_registration.rs` - add codex_fspec_tool()
- `codelet/tools/src/facade/bridge_registration.rs` - add codex_bridge_tool()
