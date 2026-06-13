# AST Research — pause-schedule / resume-schedule port (RPC-254 / RPC-292)

## Scope
Port the TypeScript `pauseSchedule` / `resumeSchedule` commands
(`src/commands/schedule/pause-schedule.ts`) to Rust, replacing the
`NotYetPorted` stubs at:
- `codelet/fspec-core/src/commands/pause_schedule.rs`
- `codelet/fspec-core/src/commands/resume_schedule.rs`

## TS source — entry points (AstGrep)
Pattern: `export async function $NAME($$$ARGS): Promise<ScheduleOperationResult> { $$$BODY }`
- `src/commands/schedule/pause-schedule.ts:23` → `pauseSchedule(options: ScheduleNameOptions)`
- `src/commands/schedule/pause-schedule.ts:51` → `resumeSchedule(options: ScheduleNameOptions)`

Both functions:
1. `cwd = options.cwd || process.cwd()`
2. `schedulesFile = getSchedulesFilePath(cwd)` → `<cwd>/spec/schedules.json`
3. `fileManager.transaction<SchedulesData>(schedulesFile, async data => { ... })`
   - lookup `data.schedules[options.name]`
   - if missing → `throw new Error("Schedule '<name>' does not exist")`
   - pause: if `status === 'paused'` → `throw "...is already paused"`; else set `status = 'paused'`
   - resume: if `status === 'active'` → `throw "...is already active"`; else set `status = 'active'`
4. returns `{ success: true }`

## TS write semantics (file-manager.ts)
- `src/utils/file-manager.ts:349-354`: on missing file, `transaction` sets `data = {}`
  then runs the callback. Accessing `data.schedules[name]` on `{}` would throw a
  TypeError in TS. **Rust DIVERGES (supervisor-APPROVED)**: model `schedules` with
  `#[serde(default)]` so a missing/empty file yields the clean
  `"Schedule '<name>' does not exist"` error instead of a crash.
- `src/utils/file-manager.ts:361`: `writeFile(tempFile, JSON.stringify(data, null, 2))`
  → 2-space indent, **NO trailing newline**. Use `io::locked_file::write_json_atomic`
  (NOT the `_trailing_newline` variant).

## Reference Rust port shape (AstGrep on list_schedules.rs)
Pattern: `struct $NAME { $$$FIELDS }`
- `list_schedules.rs:43` `ListSchedulesArgs` — `#[serde(default, rename_all="camelCase")]`
- `list_schedules.rs:61` `SchedulesFile { schedules: IndexMap<String, serde_json::Value> }`
  — the canonical on-disk model: `IndexMap` preserves insertion order; entry values
  kept as `serde_json::Value` so unknown fields round-trip verbatim.
- `list_schedules.rs:76` `ListSchedulesResult` — `#[derive(Serialize)]` for ordered output.

## Planned Rust model (mutation variant)
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Args { name: String }

#[derive(Debug, Serialize, Deserialize)]
struct SchedulesData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(default)]
    schedules: IndexMap<String, serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}
```
- Load existing `spec/schedules.json` (read-or-default-empty, mirroring TS transaction).
- Mutate `schedules[name]["status"]`.
- `write_json_atomic(spec/schedules.json, &data)`.
- Return `{ "success": true }`.

## Shared-file wiring required (supervisor-only)
- `canonical.rs::PORTED_COMMANDS` += `"pause-schedule"`, `"resume-schedule"`.
- `dispatch.rs::run_ported` += arms; remove the two `run_stub` arms (lines ~582, ~648).
- `help/configs/mod.rs` register both CONFIGs.
- `main.rs`: `Mode::PauseSchedule { name }`, `Mode::ResumeSchedule { name }` +
  `forward!` arms + `intercept_ts_help` arms + `mod pause_schedule; mod resume_schedule;`.
