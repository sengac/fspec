# AST research — `clear-virtual-hooks` (RPC-205)

## Sources read

- `src/commands/clear-virtual-hooks.ts` (92 LOC)
- `src/commands/clear-virtual-hooks-help.ts` (rich help config)
- `src/hooks/script-generation.ts::cleanupVirtualHookScript` — best-effort
  unlink of `spec/hooks/.virtual/<workUnitId>-<hookName>.sh` (ignore ENOENT).
- `src/types/index.ts` — `VirtualHook { name, event, command, blocking,
  gitContext? }` (consumed via `wu.extra["virtualHooks"]` in Rust).
- `codelet/fspec-core/src/commands/list_virtual_hooks.rs` — established
  pattern for reading `virtualHooks` from `WorkUnit.extra`.
- `codelet/fspec-core/src/commands/remove_dependency.rs` — established
  mutation+atomic-write pattern for `work-units.json`.

## TS behaviour observations

1. **Entry point** — `clearVirtualHooks({ workUnitId, cwd? })` →
   `{ success: true, clearedCount }`. No `--format=json` option (text-only
   wrapper in commander).
2. **Load** — `ensureWorkUnitsFile(cwd)` (auto-creates empty store on
   ENOENT; parse errors escalate via the helper).
3. **Validation** — `if (!data.workUnits[options.workUnitId]) throw`
   `Error("Work unit '<id>' does not exist")` — single-quoted id, exact
   substring match contract.
4. **Counted** — `clearedCount = workUnit.virtualHooks?.length || 0`.
   Missing field OR empty array both count as zero.
5. **Script cleanup** — `for (const hook of workUnit.virtualHooks)`
   call `cleanupVirtualHookScript({ workUnitId, hookName: hook.name,
   projectRoot: cwd })` inside `try { … } catch { /* ignore */ }`.
   Best-effort — never blocks the clear.
6. **Mutation** — `workUnit.virtualHooks = []` (empty array, NOT
   `delete`). `workUnit.updatedAt = new Date().toISOString()`.
7. **Persist** — `fileManager.transaction(workUnitsFile, …)` → atomic
   write. Rust equivalent: `write_json_atomic(spec/work-units.json, data)`.
8. **Stdout** — CLI prints `✓ Cleared <n> virtual hook(s) from <id>`
   (chalk.green, identity when piped). Errors print
   `✗ Failed to clear virtual hooks: <message>` (chalk.red), exit code 1.

## Rust port plan

- Single source of truth at `codelet/fspec-core/src/commands/clear_virtual_hooks.rs`:
  ```rust
  pub async fn run(args_json: &str, project_root: &Path)
      -> Result<String, FspecCoreError>;
  ```
- `Args { workUnitId: String }` — required positional; missing field →
  InvalidArgs with parse failure reason.
- Read `wu.extra["virtualHooks"]` as `Value::Array`; `cleared_count` =
  `arr.len()` (or 0 if missing).
- Iterate hook names (`v["name"].as_str()`) and best-effort
  `unlink(spec/hooks/.virtual/<id>-<name>.sh)` — ignore any ENOENT or
  other unlink errors (mirrors TS try/catch).
- Replace `wu.extra["virtualHooks"]` with `Value::Array(Vec::new())`
  (empty array, NOT remove).
- Bump `wu.updated_at = iso8601_now()`.
- `write_json_atomic(spec/work-units.json, &data)`.
- Result struct `{ success: bool, clearedCount: u64 }` — JSON serialize
  with `#[derive(Serialize)]` + `#[serde(rename = "clearedCount")]` to
  preserve insertion-order parity.
- Text rendering returned to caller: `format!("✓ Cleared {n} virtual
  hook(s) from {id}")` (no trailing newline; CLI bridge adds it).

## CLI surface (clap)

- `fspec clear-virtual-hooks <workUnitId>` — positional argument
  (required `String`). No flags. No `--format`.
- Help intercept arm prints byte-exact fixture captured from
  `node dist/index.js clear-virtual-hooks --help`.

## Open questions

- None — TS behaviour is unambiguous. Best-effort script cleanup keeps
  the clear command from failing on filesystem errors.
