# AST Research — remove-schedule (RPC-280)

Phase A/B discovery. AST analysis of the TS source to port and the Rust
reference code reused by the port.

## TS source under port

- `src/commands/schedule/remove-schedule.ts:23` — `export async function removeSchedule(options: ScheduleNameOptions): Promise<ScheduleOperationResult>`
  - Uses `getSchedulesFilePath(cwd)` directly — does NOT call `ensureSchedulesFile` (no auto-create).
  - Opens via `fileManager.transaction<SchedulesData>`:
    - if `!data.schedules[options.name]` → throw `Schedule '<name>' does not exist`
    - else `delete data.schedules[options.name]`
  - Returns `{ success: true }`.
  - CLI: positional `<name>` argument (no flags). Success prints `✓ Schedule '<name>' removed successfully`.
- `src/utils/ensure-schedules-file.ts:47` — `getSchedulesFilePath` returns `spec/schedules.json`.
- `src/types/schedule.ts` — `ScheduleNameOptions { name, cwd? }`, `SchedulesData`.
- `src/utils/file-manager.ts:361` — `transaction` writes `JSON.stringify(data, null, 2)` (NO trailing newline). On a missing file the transaction read yields the default empty map, so the not-found branch fires (supervisor APPROVED clean `does not exist` error rather than a TS crash for the missing-file case — see orchestration-state.md).

## Rust reference / reuse targets

- `codelet/fspec-core/src/commands/list_schedules.rs` — SchedulesFile = `IndexMap<String, serde_json::Value>`; insertion-order preserved on read.
- `codelet/fspec-core/src/io/locked_file.rs:92` `write_json_atomic` (2-space, no trailing newline) — the write after deletion.
- `codelet/fspec-core/src/commands/remove_command_from_foundation.rs` — sibling remove-twin shape (load → validate exists → mutate map → write).

## Parity notes / risks

- Removal preserves insertion order of remaining entries: model as `IndexMap` and use `shift_remove`/`swap_remove`? Use `IndexMap::shift_remove` to preserve order of the survivors (NOT `swap_remove`, which reorders).
- SchedulesData lives in shared `codelet/fspec-core/src/types/schedule.rs` (Phase C) with `#[serde(flatten)] extra` for unknown top-level fields; reused by add-schedule (RPC-191).
- No cron/timezone validation needed here — remove is a pure key delete.
