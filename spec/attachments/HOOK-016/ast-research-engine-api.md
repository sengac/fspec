# AST Research: Lifecycle Hook Engine API for HOOK-016

## Engine Public API (codelet/core/src/lifecycle_hooks/engine.rs)

All functions receive `CompiledLifecycleHooks` and `HookContext`:

| Function | Signature | Return Type |
|---|---|---|
| `run_session_start` | `(hooks, ctx, source)` | `SessionStartOutcome` |
| `run_session_end` | `(hooks, ctx, reason)` | `SessionEndOutcome` |
| `run_user_prompt` | `(hooks, ctx, prompt)` | `UserPromptOutcome` |
| `run_pre_tool` | `(hooks, ctx, tool_name, tool_input)` | `PreToolOutcome` |
| `run_post_tool` | `(hooks, ctx, tool_name, tool_input, tool_response)` | `PostToolOutcome` |
| `run_notification` | `(hooks, ctx, notification_type, title, message)` | `NotificationOutcome` |

## Executor (codelet/core/src/lifecycle_hooks/executor.rs)

| Function | Line | Purpose |
|---|---|---|
| `execute_command` | 23 | Core async command executor with stdin piping, env vars, timeout |

## Key Observations for HOOK-016
- All engine functions already implemented and tested in HOOK-015
- HOOK-016 tests focus on specific behavioral scenarios: prompt blocking, sequential execution, session_end payload, notification payload
- The engine iterates over `hooks.user_prompt_submit`, `hooks.notification`, etc. — sequential by default
- For pre/post_tool_use, the engine iterates over groups and then over `group.commands` — this is the sequential execution pattern to test
