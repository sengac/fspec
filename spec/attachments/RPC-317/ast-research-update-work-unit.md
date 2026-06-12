# AST Research — `update-work-unit` (RPC-317)

## TS source of truth
- `src/commands/update-work-unit.ts` (222 LOC)
- `src/commands/update-work-unit-help.ts` (help config)

## Public surface
Commander.js registration (`registerUpdateWorkUnitCommand`):
- positional `<workUnitId>` (required)
- `-t, --title <title>`
- `-d, --description <description>`
- `-e, --epic <epic>`
- `-p, --parent <parent>`

NOTE: there is NO `--type` flag on the CLI surface. `type` only enters
through the dispatcher/option object (`UpdateWorkUnitOptions.type`) and is
ALWAYS rejected as immutable (see Behaviour 2). On CLI it is unreachable, but
the dispatcher arg shape must still accept `type` and produce the immutable
error.

## Core function `updateWorkUnit(options)` behaviours (one rule each)

1. **Existence check** — `if (!workUnitsData.workUnits[id]) throw "Work unit '<id>' does not exist"`. (line 33-35)
2. **Type immutability** — if `options.type !== undefined` → throw a multi-line
   error: `"Work unit type is immutable and cannot be changed after creation.\n\nCurrent type: <type||'story'>\nAttempted to change to: <type>\n\nIf you need to change the type, Delete this work unit and create a new one with the correct type."` (lines 38-45)
3. **Parent existence** — if `options.parent` truthy and parent not in workUnits → throw `"Parent work unit '<parent>' does not exist"`. (lines 48-51)
4. **Circular reference** — `wouldCreateCircularReference(...)` walks `parent`
   chain; if cycle → throw `"Circular parent relationship detected"`. Self-parent
   (`proposedParentId === workUnitId`) counts as circular. (lines 53-63, 149-184)
5. **Epic existence** — if `options.epic !== undefined` → read epics via
   `ensureEpicsFile`; if epic not in `epicsData.epics` → throw `"Epic '<epic>' does not exist"`. (lines 65-72)
6. **Title update** — if `options.title !== undefined` set `wu.title`. (75-77)
7. **Description update** — if `options.description !== undefined` set `wu.description`. (79-82)
8. **Epic move** — if `options.epic !== undefined`: set `wu.epic`; then
   `fileManager.transaction(epicsFile)`:
     - remove id from OLD epic's `workUnits` array (filter) if old epic exists & has array
     - ensure new epic has `workUnits` array; push id if not already present. (84-108)
9. **Parent move** — if `options.parent !== undefined`:
     - remove id from OLD parent's `children` array (filter) if old parent exists & has array
     - set `wu.parent = options.parent`
     - ensure new parent has `children` array; push id if not already present. (110-135)
10. **Timestamp** — always set `wu.updatedAt = new Date().toISOString()`. (137-139)
11. **Atomic write** — `fileManager.transaction(workUnitsFile, data => Object.assign(data, workUnitsData))`. (141-144)
12. **Result** — `{ success: true }`. (146)

### Edge notes
- `options.parent` is checked with truthiness `if (options.parent)` for existence/circular (lines 48), but the MUTATION uses `if (options.parent !== undefined)` (line 110). So an empty-string parent passes the existence guard (skipped) but still triggers mutation branch — in practice parent is required-value when present. Mirror TS: existence check on truthy, mutation on `!== undefined`.
- Epic move uses a SEPARATE transaction on epics.json BEFORE the work-units.json write. Two atomic writes.
- `wu.epic` field is set on the in-memory object before the epics.json transaction reads `oldEpic` from `wu.epic` — but oldEpic is captured at line 85 (`const oldEpic = wu.epic`) BEFORE reassigning at line 86. Capture-then-set ordering matters.
- `parent` mutation: oldParent captured from `wu.parent` (line 112) BEFORE setting new parent (line 122).

## CLI output (registerUpdateWorkUnitCommand)
- success: `chalk.green("✓ Work unit <id> updated successfully")`
- error: `chalk.red("✗ Failed to update work unit:"), error.message` + `process.exit(1)`

## Rust mapping plan
- Core: `commands/update_work_unit.rs` — `pub async fn run(args_json, project_root)`.
- Args struct: `workUnitId, title?, description?, epic?, parent?, type?` (camelCase serde).
- Load work-units via `ensure_work_units_file` (auto-create parity).
- Epic existence via `ensure_epics_file` (TS uses ensureEpicsFile → auto-create).
- Two atomic writes: epics.json (on epic move) then work-units.json.
- `wu.epic` / `wu.parent` / `wu.title` / `wu.description` / `children` arrays live partly in `extra` map — title is typed, epic typed, parent/children/description in extra.
  - `title`, `epic`, `updated_at` are typed on `WorkUnit`.
  - `description`, `parent`, `children` live in `extra` (NOT typed) — mutate via extra map by string key, matching update_prefix.rs pattern.
- Circular-reference walk ported as a recursive helper over the IndexMap.
- Result JSON: `{ "success": true }` pretty-printed (matches update_prefix.rs).
- Error wrapping: TS errors thrown raw (no outer-catch wrap in core); CLI registration adds the `✗ Failed to update work unit:` prefix at the OUTPUT layer only. So the FspecCoreError reason = raw message; CLI bridge prints `Error: <reason>` style (parity question — TS prints `✗ Failed to update work unit: <msg>`). Decide bridge output to mirror TS chalk text.

## Shared-file asks for supervisor
- `ensure_epics_file` already exists in io/ensure.rs — reuse, no change.
- `ensure_work_units_file` exists — reuse.
- May need to add `update-work-unit` to PORTED_COMMANDS / dispatch / mod.rs / main.rs / help configs mod (ALL supervisor-owned).
