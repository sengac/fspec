# AST Research — RPC-250 list-schedules Rust port

## Reference port signature (list_hooks)

Searched: `pub async fn run($$$ARGS) -> Result<String, FspecCoreError>` in `codelet/fspec-core/src/commands/list_hooks.rs`.

```
codelet/fspec-core/src/commands/list_hooks.rs:109:
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

This is the canonical shape for ported commands (RPC-247). list_schedules must adopt the same signature.

## Current stub signature (list_schedules)

```
codelet/fspec-core/src/commands/list_schedules.rs:6:
pub async fn run(_args_json: &str) -> Result<String, FspecCoreError>
```

The stub is missing the `project_root: &Path` argument and returns `NotYetPorted`. Phase C must rewrite this module to mirror list_hooks::run (RPC-247), reading `spec/schedules.json` from `project_root` rather than `std::env::current_dir()`.

## TS source-of-truth

`src/commands/schedule/list-schedules.ts` reads `spec/schedules.json` via `fileManager.readJSON(file, defaultData)` with `defaultData = { version: '1.0.0', schedules: {} }`. The default-on-missing-or-invalid semantics are what we are porting. Hard-coded `columns: ["name","cron","timezone","type","status","lastRun","nextRun"]` is emitted on both happy and error paths.

## Conclusion

Phase C must:
1. Replace the stub with `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` (matching list_hooks::run).
2. Use `IndexMap<String, serde_json::Value>` to preserve insertion order and pass entries through verbatim.
3. Swallow both ENOENT and serde_json parse errors → canonical empty payload with columns.
4. Support `format: "json" | "text"` (default text).
