# REFAC-046 AST research

`grep '^impl|^pub fn|^async fn' rust/tools/src/unified_exec/tool.rs`: `struct UnifiedExecTool` (31),
`impl UnifiedExecTool` builder (39-73), `impl Tool for UnifiedExecTool` (79-159), action handlers
`impl UnifiedExecTool` (165-407: handle_run 167, handle_write 290, handle_poll 331, handle_list 351, handle_close
370), shared polling functions (409-529). Callers reach the handlers only through `Tool::call`, so moving the impl
block into a child module changes no paths.

Coverage files mapping `unified_exec/tool.rs`: unified-exec-tool, compaction-builder-subprocess-cancellation,
exec-stdin-prompt.
