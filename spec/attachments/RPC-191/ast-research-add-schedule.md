# AST Research — add-schedule (RPC-191)

Phase A/B discovery. AST analysis of the TS source to port and the Rust
reference code reused by the port.

## TS source under port

- `src/commands/schedule/add-schedule.ts:54` — `export async function addSchedule(options: AddScheduleOptions): Promise<AddScheduleResult>`
  - Validation order (load-bearing for parity):
    1. `validateScheduleName` — slug regex `^[a-z0-9]+(-[a-z0-9]+)*$` (trimmed)
    2. `validateCronExpression` (src/utils/validators/cron.ts) — split on `\s+`, require exactly 5 fields, then `cron-validate` with 5-field preset
    3. `validateTimezone` (src/utils/validators/timezone.ts) — `Intl.supportedValuesOf('timeZone')` + aliases UTC/GMT/Etc/UTC/Etc/GMT
    4. jobType branch: `agent` requires role+prompt; `shell` requires command; else `Invalid jobType: <type>...`
    5. `ensureSchedulesFile(cwd)` — auto-create `{version:'1.0.0',schedules:{}}`
    6. duplicate check inside `fileManager.transaction`: `Schedule '<name>' already exists`
  - Entry field order written: name, cron, timezone, overlapPolicy (default 'skip'),
    status 'active', lastRunAt null, lastRunStatus null, createdAt ISO8601,
    then jobType + role/prompt (agent) or command (shell).
- `src/utils/ensure-schedules-file.ts` — `ensureSchedulesFile` + `getSchedulesFilePath`; default `{version:'1.0.0', schedules:{}}`.
- `src/types/schedule.ts` — `SchedulesData { version, schedules: Record<string, ScheduleEntry> }`; AddScheduleOptions fields.
- `src/utils/file-manager.ts:361` — `transaction` writes `JSON.stringify(data, null, 2)` (NO trailing newline) → Rust uses `write_json_atomic` (NOT the trailing-newline variant).

## Rust reference / reuse targets

- `codelet/fspec-core/src/commands/list_schedules.rs` — schedules.json modelled as `IndexMap<String, serde_json::Value>` (insertion order). Copy SchedulesFile shape.
- `codelet/core/src/scheduler/cron_utils.rs:36` `parse_cron(expr,ctx)` via `croner::Cron::new(expr).parse()`; `:46` `parse_timezone(tz_str,ctx)` via `tz_str.parse::<chrono_tz::Tz>()`. These confirm the validation primitives available — `croner` + `chrono-tz` are workspace deps NOT yet in fspec-core/Cargo.toml (Phase-C shared-file add).
- `codelet/fspec-core/src/io/locked_file.rs:37` `read_or_init_json` (auto-create) + `:92` `write_json_atomic` (2-space, no trailing newline). Basis for the Phase-C `ensure_schedules_file` helper.
- `codelet/fspec-core/src/commands/add_rule.rs` — canonical mutation port shape (validate → mutate extra map → write_json_atomic).

## Parity notes / risks

- croner may accept 6-field (with-seconds) crons; enforce the TS 5-field count check FIRST (`split_whitespace().count() == 5`) before `croner` parse.
- chrono-tz `Tz` FromStr accepts IANA names but NOT bare "GMT"/"UTC" aliases the same way as Intl — verify UTC parses (it does: `chrono_tz::UTC`). Document any divergence at Phase C.
- SchedulesData → new `codelet/fspec-core/src/types/schedule.rs` (supervisor decision) with `#[serde(flatten)] extra` to round-trip unknown top-level fields.
