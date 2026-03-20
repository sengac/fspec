# AST Research: pre_tool_use Short-Circuit Logic

## run_pre_tool (engine.rs:202-272)
- Iterates over `hooks.pre_tool_use` groups
- For each group, checks `group.matcher.matches(tool_name)`
- For each matching command, calls `execute_command()`
- Interprets result via `interpret_pre_tool_result()`
- On Allow/Deny → returns immediately (short-circuit)
- On Continue → continues to next command/group
- Default return: Continue (if all groups return Continue)

## Key Types
- `PreToolOutcome { decision, reason, messages }`
- `PreToolHookDecision { Allow, Deny, Continue }`
- `CompiledPreToolUseGroup { matcher, commands }`
- `HookContext { session_id, cwd, transcript_path }`

## interpret_pre_tool_result (response.rs)
- Exit code 0 + JSON `permissionDecision: "allow"` → Allow
- Exit code 2 OR JSON `permissionDecision: "deny"` → Deny
- Exit code 0 without permissionDecision → Continue
- Timeout → Deny (safety-first)
