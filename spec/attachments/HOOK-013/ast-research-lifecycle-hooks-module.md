# AST Research: Lifecycle Hooks Module Structure (HOOK-013 Parent Review)

## Module Layout (mod.rs)
```
pub mod compiled      — Compiled hook types with regex matchers
pub mod config        — Serde deserialization types
pub mod engine        — Session lifecycle hook runners (start/end/prompt/notification)
pub(crate) mod executor  — Low-level process execution
pub(crate) mod helpers   — Shared utilities
pub mod loader        — Two-level config loading and merging
pub mod outcome       — Outcome/decision enums
pub mod payloads      — Per-event JSON payload structs
pub(crate) mod response  — Output interpretation
pub mod tool_engine   — Tool-specific hook runners (pre/post_tool_use)
```

## Public API Functions
```rust
// engine.rs — session lifecycle
pub async fn run_session_start(hooks, ctx) -> SessionHookOutcome
pub async fn run_session_end(hooks, ctx, reason) -> SessionHookOutcome
pub async fn run_user_prompt(hooks, ctx, prompt) -> UserPromptHookOutcome
pub async fn run_notification(hooks, ctx, level, message, source) -> SessionHookOutcome

// tool_engine.rs — tool use lifecycle
pub async fn run_pre_tool(hooks, ctx, tool_name, tool_input) -> PreToolHookOutcome
pub async fn run_post_tool(hooks, ctx, tool_name, tool_input, tool_response) -> SessionHookOutcome

// executor.rs — internal
pub(crate) async fn execute_command(cmd, timeout, shell, payload, env) -> CommandResult

// loader.rs — config loading
pub fn load_lifecycle_hooks(workspace, user_config_dir) -> Result<Option<CompiledLifecycleHooks>>
```

## File Sizes (all under 300 lines)
```
 92 compiled.rs
 97 config.rs
251 engine.rs
159 executor.rs
 55 helpers.rs
272 loader.rs
 39 mod.rs
 75 outcome.rs
 71 payloads.rs
129 response.rs
150 tool_engine.rs
---
1390 total
```

## Test Coverage: 44 tests across 4 test files
- lifecycle_hooks_config_test.rs: 15 tests (HOOK-014)
- lifecycle_hooks_engine_test.rs: 19 tests (HOOK-015)
- lifecycle_hooks_session_test.rs: 6 tests (HOOK-016)
- lifecycle_hooks_tool_test.rs: 4 tests (HOOK-017)

## Feature Coverage: 42/42 scenarios (100%)
