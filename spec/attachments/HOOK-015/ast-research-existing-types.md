# AST Research: HOOK-015 Existing Codebase Types

## Existing Lifecycle Hook Types (from HOOK-014)

### Public Functions
- `load_lifecycle_hooks(project_root, user_home) -> Result<Option<CompiledLifecycleHooks>>` — loader.rs:32

### Config Types (config.rs)
- `FspecHooksConfig` — top-level config with global + hooks HashMap
- `GlobalConfig` — timeout, shell
- `HookDefinition` — name, command, blocking, timeout
- `HookGroupConfig` — matcher, hooks Vec<HookCommandConfig>
- `HookCommandConfig` — command, timeout

### Compiled Types (compiled.rs)
- `CompiledLifecycleHooks` — per-event Vec fields (session_start, etc.), global_timeout, global_shell
- `CompiledHookDefinition` — name, command, blocking, timeout (resolved)
- `CompiledHookGroup` — matcher: HookMatcher, commands: Vec<CompiledHookCommand>
- `CompiledHookCommand` — command, timeout (resolved)
- `HookMatcher` enum — Any | Pattern(Regex)

### No Existing Process Execution
- No `tokio::process::Command` usage found in lifecycle_hooks module
- Engine (HOOK-015) will add async child process execution
- Will use CompiledLifecycleHooks as input, produce typed Outcome structs
