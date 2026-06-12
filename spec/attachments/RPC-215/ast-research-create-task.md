# AST Research — RPC-215 Port `create-task` TS → Rust

## TypeScript source surface (`src/commands/create-task.ts`)

Grep `function (createTask|generateNextId|calculateNestingDepth|registerCreateTaskCommand)`:

- `export async function createTask(...)` — `:32` — core domain logic (validation → mutation → systemReminder).
- `function generateNextId(workUnitsData, prefix): string` — `:172` — high-water-mark id generation.
- `function calculateNestingDepth(...)` — `:202` — recursive parent-walk for the max-depth (3) guard.
- `export async function createTaskCommand(...)` — `:215` — CLI action callback (renders `output.log` success block + `output.error(systemReminder)`).
- `export function registerCreateTaskCommand(program)` — `:273` — Commander.js registration (prefix, title, -d/--description, -e/--epic, -p/--parent).

## Rust port target surface (`codelet/fspec-core/src/commands/create_task.rs`)

- `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` — `:76`
  - single dispatcher + CLI entry point (two-front-doors, RPC-003 §7/§11).
- Helper fns mirrored from TS: `generate_next_id`, `nesting_depth`, `system_reminder`, `render_success`, `read_raw_work_units_object`, `read_raw_epics_object`.

## Difference vs create-bug

`type: "task"` instead of `"bug"`; the system-reminder body is the operational-work /
minimal-requirements text ("Tasks can move directly to implementing without specifying phase.")
rather than the bug research-guidance block. Validation, id generation, nesting-depth guard,
and atomic-write mutation are otherwise identical.

## Shared helpers consumed (supervisor-owned `io/ensure.rs`)

`check_foundation_exists`, `ensure_prefixes_file`, `ensure_work_units_file`, `ensure_epics_file`
(auto-creating). Identical import line shared with create-bug and Worker 4's create_story.

## Conclusion

1:1 structural port. All key risk areas (field order, high-water-mark, nesting guard, verbatim
task system-reminder) are covered by dedicated scenarios in `create-task-rust-port.feature`.
