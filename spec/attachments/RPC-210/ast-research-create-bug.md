# AST Research — RPC-210 Port `create-bug` TS → Rust

## TypeScript source surface (`src/commands/create-bug.ts`)

AstGrep / Grep `function (createBug|generateNextId|calculateNestingDepth|registerCreateBugCommand)`:

- `export async function createBug(...)` — `:32` — core domain logic (validation → mutation → systemReminder).
- `function generateNextId(workUnitsData, prefix): string` — `:177` — high-water-mark id generation.
- `function calculateNestingDepth(...)` — `:207` — recursive parent-walk for the max-depth (3) guard.
- `export async function createBugCommand(...)` — `:220` — CLI action callback (renders `output.log` success block + `output.error(systemReminder)`).
- `export function registerCreateBugCommand(program)` — `:278` — Commander.js registration (prefix, title, -d/--description, -e/--epic, -p/--parent).

## Rust port target surface (`codelet/fspec-core/src/commands/create_bug.rs`)

AstGrep `pub async fn run($$$ARGS) -> $RET { $$$BODY }`:

- `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` — `:76`
  - single dispatcher + CLI entry point (two-front-doors, RPC-003 §7/§11).
- Helper fns mirrored from TS: `generate_next_id`, `nesting_depth`, `system_reminder`, `render_success`, `read_raw_work_units_object`, `read_raw_epics_object`.

## Reference impl shape (`create_epic.rs`)

AstGrep `fn $NAME($$$ARGS) -> $RET { $$$BODY }`:

- `fn is_valid_epic_id`, `fn render_success`, `fn read_raw_epics_object` — confirms the
  create-* convention: typed `*Args` (serde camelCase), ordered `serde_json::Map` build to
  preserve TS object-literal key order, raw round-trip read to preserve existing record order,
  `write_json_atomic` for the persisted mutation.

## Shared helpers consumed (supervisor-owned `io/ensure.rs`)

`check_foundation_exists`, `ensure_prefixes_file`, `ensure_work_units_file`, `ensure_epics_file`
(auto-creating). Converged on identical import line with Worker 4 (`create_story`).

## Conclusion

The port is a 1:1 structural mapping. No new domain abstractions required; key risk areas
(on-disk field order, high-water-mark, nesting-depth guard, verbatim system-reminder) are all
covered by dedicated scenarios in `create-bug-rust-port.feature`.
