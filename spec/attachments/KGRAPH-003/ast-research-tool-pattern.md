# AST Research: Tool Registration Pattern for GraphSearch

## Session-Aware Tool Structs (all tools with session_id)

Found 15 tool structs with `session_id: Uuid`:
- `codelet/tools/src/session_search/mod.rs:47` — SessionSearchTool
- `codelet/tools/src/agent_manager/mod.rs:47` — AgentManagerTool
- `codelet/tools/src/inject_summary.rs:112` — InjectSummaryTool
- `codelet/tools/src/schedule/mod.rs:62` — ScheduleTool
- `codelet/tools/src/read.rs:111` — ReadTool
- `codelet/tools/src/write.rs:22` — WriteTool
- `codelet/tools/src/edit.rs:24` — EditTool
- `codelet/tools/src/grep.rs:56` — GrepTool
- `codelet/tools/src/ls.rs:42` — LsTool
- `codelet/tools/src/glob.rs:29` — GlobTool
- `codelet/tools/src/astgrep.rs:30` — AstGrepTool
- `codelet/tools/src/astgrep_refactor.rs:116` — AstGrepRefactorTool
- `codelet/tools/src/unified_exec/tool.rs:25` — UnifiedExecTool
- `codelet/tools/src/apply_patch/mod.rs:63` — ApplyPatchTool
- `codelet/tools/src/request_user_input.rs:253` — RequestUserInputTool

## Handler Registration Pattern (SessionSearch as reference)

Handler map: `codelet/tools/src/session_search/handler.rs:41`
```rust
pub fn set_session_search_handler(session_id: Uuid, handler: Option<SessionSearchHandler>)
```

## Conclusion

GraphSearchTool should follow exact same pattern:
1. Struct with `session_id: Uuid` in `codelet/tools/src/graph_search/mod.rs`
2. Handler type + global map in `codelet/tools/src/graph_search/handler.rs`
3. Concrete handler factory in `codelet/napi/src/graph_search_handler.rs`
4. Registration in session_manager.rs alongside SessionSearch registration
