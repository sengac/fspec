# AST research HOOK-018: existing hook code

## pattern: pub fn run_for_transition($$$ARGS) -> $RET { $$$BODY }
```
rust/fspec-core/src/virtual_hooks_exec.rs:34:1:pub fn run_for_transition(
```
## pattern: pub(super) fn run_pre_hooks($$$ARGS) -> $RET { $$$BODY }
```
rust/fspec-core/src/commands/update_work_unit_status/board_state.rs:70:1:pub(super) fn run_pre_hooks(
```
## pattern: pub fn dispatch_command($$$ARGS) -> $RET { $$$BODY }
```
rust/fspec-core/src/dispatch.rs:133:1:pub fn dispatch_command(req: DispatchRequest) -> DispatchResult {
```

Findings: virtual pre-<status> hooks run in update_work_unit_status/board_state.rs run_pre_hooks; virtual_hooks_exec::run_for_transition (post-<status>) has no caller; dispatch_command has no command-hook phase; rust/core/src/lifecycle_hooks reads the same fspec-hooks.json only for agent lifecycle events (config.rs: fspec CLI events ignored). TS reference: src/hooks/{config,conditions,discovery,executor,formatting,command-utils}.ts at 103a73072^ and src/hooks/integration.ts at e69af9997^.
