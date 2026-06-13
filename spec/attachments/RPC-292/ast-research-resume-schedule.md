# AST Research — resume-schedule port (RPC-292)

## Scope
Port the TypeScript `resumeSchedule` command
(`src/commands/schedule/pause-schedule.ts:51-71`) to Rust, replacing the
`NotYetPorted` stub at `codelet/fspec-core/src/commands/resume_schedule.rs`.
Twin of RPC-254 (pause-schedule) — see
`spec/attachments/RPC-254/ast-research-pause-schedule.md` for the shared analysis.

## TS source — entry point (AstGrep)
Pattern: `export async function $NAME($$$ARGS): Promise<ScheduleOperationResult> { $$$BODY }`
- `src/commands/schedule/pause-schedule.ts:51` → `resumeSchedule(options: ScheduleNameOptions)`

Behaviour (`pause-schedule.ts:51-71`):
1. `cwd = options.cwd || process.cwd()`; `getSchedulesFilePath(cwd)` → `<cwd>/spec/schedules.json`
2. `fileManager.transaction<SchedulesData>` callback:
   - `schedule = data.schedules[options.name]`
   - if missing → `throw new Error("Schedule '<name>' does not exist")`
   - if `schedule.status === 'active'` → `throw new Error("Schedule '<name>' is already active")`
   - else `schedule.status = 'active'`
3. returns `{ success: true }`

CLI registration (`pause-schedule.ts:89-103`): `program.command('resume-schedule')`,
`.argument('<name>', 'Schedule name to resume')`, on success
`output.log("✓ Schedule '<name>' resumed successfully")`, on error
`output.error("✗ Failed to resume schedule:", message); process.exit(1)`.

## Differences from pause-schedule (RPC-254)
| Aspect            | pause-schedule           | resume-schedule          |
|-------------------|--------------------------|--------------------------|
| target status     | `paused`                 | `active`                 |
| already-X guard   | `is already paused`      | `is already active`      |
| success message   | `paused successfully`    | `resumed successfully`   |
| help config       | `pause-schedule-help.ts` | `resume-schedule-help.ts`|

Everything else (on-disk model, write semantics, missing-file APPROVED divergence,
shared-file wiring) is identical — reuse the same `SchedulesData` shape.

## Write semantics
- `file-manager.ts:361`: `JSON.stringify(data, null, 2)` → 2-space, NO trailing newline →
  `io::locked_file::write_json_atomic`.
- Missing/empty file → `#[serde(default)]` empty `schedules` map → clean
  `"Schedule '<name>' does not exist"` (supervisor-APPROVED divergence from TS TypeError).

## Reference Rust port (AstGrep on list_schedules.rs)
- `list_schedules.rs:61` `SchedulesFile { schedules: IndexMap<String, serde_json::Value> }`
  — copy this on-disk model shape (IndexMap + Value entries for verbatim round-trip).
