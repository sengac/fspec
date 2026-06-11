# AST research — `copy-virtual-hooks` (RPC-209)

## Sources read

- `src/commands/copy-virtual-hooks.ts` (131 LOC)
- `src/commands/copy-virtual-hooks-help.ts` (rich help config)
- `src/types/index.ts` — `VirtualHook { name, event, command, blocking,
  gitContext? }` (read via `wu.extra["virtualHooks"]` array).
- `codelet/fspec-core/src/commands/list_virtual_hooks.rs` — virtualHooks
  read pattern (extra flatten map).
- `codelet/fspec-core/src/commands/remove_dependency.rs` — pattern for
  load → mutate → write_json_atomic over `work-units.json`.

## TS behaviour observations

1. **Entry point** — `copyVirtualHooks({ from, to, hookName?, cwd? })`
   → `{ success: true, copiedCount }`. Commander wraps with `--from`,
   `--to`, `--hook-name` options.
2. **CLI guards** — Commander action throws `"--from option is required"`
   / `"--to option is required"` BEFORE calling the core function. Mirror
   these in the Rust CLI bridge.
3. **Load** — `ensureWorkUnitsFile(cwd)`.
4. **Validation pre-flight (in order)**:
   1. `!data.workUnits[from]` → `Error("Source work unit '<from>' does not exist")`.
   2. `!data.workUnits[to]` → `Error("Target work unit '<to>' does not exist")`.
   3. `!sourceWorkUnit.virtualHooks || length === 0` →
      `Error("No virtual hooks configured for source work unit <from>")`
      (note: NO single quotes around id here, mirror the TS literal exactly).
5. **Select hooks**:
   - If `hookName` provided → `find(h => h.name === hookName)`.
     Miss → `Error("Hook '<hookName>' not found in <from>")` (single
     quotes around hook name, none around from id).
   - Otherwise copy all hooks.
6. **Deep copy** — `hooksToCopy.map(hook => ({ ...hook }))`. Rust:
   `hook.clone()` on `Value` objects (each entry's `Map`).
7. **Append to target** — `targetWorkUnit.virtualHooks ||= []`; then
   `.push(...copiedHooks)`. Existing hooks on target are preserved;
   copied hooks are appended at the end.
8. **Update timestamp on TARGET only** — `targetWorkUnit.updatedAt =
   new Date().toISOString()`. Source unit's `updatedAt` is NOT bumped.
9. **Persist** — `fileManager.transaction(workUnitsFile, …)` → atomic
   write. Rust equivalent: `write_json_atomic`.
10. **No script generation** — copy only the config JSON. Help note: "Script
    files for git context hooks are NOT copied (regenerated on execution)".
11. **Stdout** — `✓ Copied <n> virtual hook(s) from <from> to <to>`.
    Errors → `✗ Failed to copy virtual hooks: <message>`, exit code 1.

## Rust port plan

- Single source of truth at `codelet/fspec-core/src/commands/copy_virtual_hooks.rs`:
  ```rust
  pub async fn run(args_json: &str, project_root: &Path)
      -> Result<String, FspecCoreError>;
  ```
- `Args { from: String, to: String, hookName: Option<String> }`
  (camelCase via `#[serde(rename_all = "camelCase")]`). All three optional
  at parse time — but `from`/`to` empty/missing surface as the explicit
  TS guard message at parse/validation time. We default to `String::new()`
  and check `is_empty()` so the CLI-side guards and core-side guards
  produce the same canonical error text.
- Read source `wu.extra["virtualHooks"]` as `Value::Array`. Empty/missing
  → return the "No virtual hooks configured for source work unit <from>"
  error.
- When `hookName` provided, iterate the array filtering by `v["name"].as_str()`.
- Append the chosen subset (deep cloned via `Value::clone()`) to the
  target's `extra["virtualHooks"]` array (initialize as `[]` if absent).
- Bump `target.updated_at = iso8601_now()`. SOURCE updatedAt NOT touched.
- `write_json_atomic(spec/work-units.json, &data)`.
- Result struct `{ success, copiedCount }`.
- Text rendering: `format!("✓ Copied {n} virtual hook(s) from {from} to {to}")`.

## CLI surface (clap)

- `fspec copy-virtual-hooks --from <id> --to <id> [--hook-name <name>]`.
- Both `--from` and `--to` are required at the clap layer; absence in
  TS shows the friendly error string ("--from option is required").
  We surface the same string via the CLI bridge before delegating to
  the core function so that exit-code 1 with that exact stderr line
  matches TS behaviour.
- Help intercept arm prints byte-exact fixture from
  `node dist/index.js copy-virtual-hooks --help`.

## Open questions

- None. The TS guards order is `from missing → to missing → source missing →
  target missing → no hooks → hook-name miss`. Rust mirrors that ordering
  so the integration tests can pin each error string in isolation.
