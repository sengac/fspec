# AST Research — discover-event-storm (RPC-225)

## TS Source
`src/commands/discover-event-storm.ts` (91 LOC) + `src/commands/discover-event-storm-help.ts`.

## Behaviour summary
`discoverEventStormCommand({ workUnitId, cwd? })`:
1. `cwd = options.cwd || process.cwd()`.
2. Reads `spec/work-units.json`. If file missing → `output.error('✗ spec/work-units.json not found. Run fspec init first.')` then `process.exit(1)`.
3. Parses as `WorkUnitsData`. If `workUnits[workUnitId]` absent → `output.error('✗ Work unit <id> not found')` then `process.exit(1)`.
4. If `workUnit.status !== 'specifying'` → TWO error lines:
   - `✗ Work unit <id> must be in specifying status (currently: <status>)`
   - `  Run: fspec update-work-unit-status <id> specifying`
   then `process.exit(1)`.
5. Otherwise: builds guidance via `getEventStormSection()` (src/utils/slashCommandSections/eventStorm.ts, ~220 lines static template string), wraps in `wrapInSystemReminder(...)` =
   `<system-reminder>\nEVENT STORM DISCOVERY - <id>\n\n<guidance>\n\nWork unit: <id>\n\nUse the commands listed above to capture Event Storm artifacts.\nWhen done, run: fspec generate-example-mapping-from-event-storm <id>\n</system-reminder>`.
6. Logs `✓ Event Storm discovery session started for <id>` (green), then logs the reminder.
7. NO file mutation — read-only command.

## Commander registration
`.command('discover-event-storm').argument('<work-unit-id>', 'Work unit ID')` — single required positional, no flags.

## Rust port plan
- File-missing behaviour: TS uses `existsSync` + explicit error, NO auto-create → use **Option B** (inline `path.exists()` check, no `ensure_work_units_file`). Mirror `add_domain_event.rs` missing-file pattern.
- Read-only: no `write_json_atomic` call.
- Read via `serde_json::from_str::<WorkUnitsData>` (escalate malformed JSON → ParseJson).
- Status check uses `WorkUnitStatus::as_str()` (parity with `add_rule.rs` specifying gate).
- Dispatcher path: errors as `FspecCoreError::InvalidArgs`. CLI path renders stderr `Error:` + exit 1.
- Success output text = green success line + `\n` + wrapped system-reminder. Returned as the `Ok(String)` from `run`.

## SHARED-CONTENT REQUEST
`getEventStormSection()` is a ~220-line static template string. NOT yet ported to Rust.
Need a shared Rust source for this guidance text (e.g. `fspec-core/src/slash_command_sections/event_storm.rs` exposing `pub fn event_storm_section() -> &'static str` or `String`). Flagging to supervisor — do NOT inline ad-hoc; ASK for a shared fn/const.

## Shared types/fns reused
- `crate::types::work_unit::WorkUnitsData` / `WorkUnitStatus::as_str()`
- `crate::error::FspecCoreError`
- system-reminder wrapping: check whether a shared `wrap_in_system_reminder` helper exists in fspec-core (create_story.rs inlines `<system-reminder>` literals). May need shared helper — flag.
