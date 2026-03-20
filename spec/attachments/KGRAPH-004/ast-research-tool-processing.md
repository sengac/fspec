# AST Research: Tool Call Processing in Session Manager

## Searched for:
- annotation detection patterns
- tool result handling in session_manager.rs
- tool_use / tool_result patterns

## Findings:
- No `detect_annotations` or `annotation_detector` found in codebase
- Tool call processing happens in session_manager.rs agent loop
- The integration point will be in the tool result handling path, where we can inspect
  tool names and args after each tool call completes

## Pattern for integration:
After a tool call result is received in the agent loop, check the tool name:
- "Write" / "Edit" → extract CodeEntity
- "Fspec" → inspect command arg → extract WorkUnit
- Queue entities for batch insert
