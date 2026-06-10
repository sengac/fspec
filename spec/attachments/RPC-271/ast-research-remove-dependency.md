# AST Research — `remove-dependency` (RPC-271)

## TS Source

- File: `src/commands/remove-dependency.ts` (201 LOC)
- Public API: `export async function removeDependency(options: RemoveDependencyOptions): Promise<RemoveDependencyResult>`
- Registers Commander subcommand via `registerRemoveDependencyCommand(program)`.

## Options shape (TS lines 9-16)

```ts
interface RemoveDependencyOptions {
  workUnitId: string;
  blocks?: string;       // single id (NOT an array like add-dependencies)
  blockedBy?: string;
  dependsOn?: string;
  relatesTo?: string;
  cwd?: string;
}
```

Result shape:
```ts
interface RemoveDependencyResult { success: boolean }
```

## Behaviour observations (per branch)

### Source-exists guard (lines 30-32)
Throws `Work unit '<id>' does not exist` when the source work unit is missing.

### `blocks` removal (lines 37-57) — bidirectional
- Filter `workUnit.blocks` removing `options.blocks` id; if resulting array empty, `delete workUnit.blocks`.
- Reverse-edge cleanup: locate `data.workUnits[options.blocks]` (target); filter `targetWorkUnit.blockedBy` removing `options.workUnitId`; if empty, delete the field.
- Target-missing case: silent no-op on the reverse edge (TS guard `if (data.workUnits[options.blocks])`).

### `blockedBy` removal (lines 60-82) — bidirectional
- Mirror of `blocks`: filter `workUnit.blockedBy`, delete if empty.
- Reverse-edge cleanup on the target's `blocks` array (filter + delete-if-empty).

### `dependsOn` removal (lines 85-94) — UNIDIRECTIONAL
- Filter `workUnit.dependsOn` removing `options.dependsOn`; delete field if empty.
- No reverse-edge action (matches add-dependency unidirectional semantics).

### `relatesTo` removal (lines 97-119) — bidirectional symmetric
- Filter `workUnit.relatesTo`; delete if empty.
- Reverse cleanup on `targetWorkUnit.relatesTo` (filter source out, delete if empty).

### Bump `updatedAt` (line 121)
- `workUnit.updatedAt = new Date().toISOString()` on the SOURCE unit only. (No `updatedAt` bump for the bidirectional target — matches TS exactly.)

### Persistence (lines 124-127)
- `fileManager.transaction(workUnitsFile, async fileData => { Object.assign(fileData, data); })`
- Equivalent in Rust: load → mutate in-memory → single `write_json_atomic` at end.

### NO cycle detection / NO status transitions
Removing a dependency NEVER changes any work unit's status nor any state-array contents. This is a critical difference from `add-dependencies` / `add-dependency`. The TS code does NOT auto-revert "blocked" → previous status when the last `blockedBy` is removed. The Rust port must mirror this.

## Commander.js surface (lines 133-200)

```
remove-dependency <workUnitId> [dependsOnId]
  --blocks <targetId>
  --blocked-by <targetId>
  --depends-on <targetId>
  --relates-to <targetId>
```

### Shorthand argument
- Positional `[dependsOnId]` (second arg) is equivalent to `--depends-on`.
- If BOTH positional and `--depends-on` supplied AND they differ → throw `Cannot specify dependency both as argument and --depends-on option`.
- If they agree → use the value (no error).

### At-least-one-flag guard (lines 173-182)
If `!finalDependsOn && !options.blocks && !options.blockedBy && !options.relatesTo` →
throw `Must specify at least one relationship to remove: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to`.

### Success line (line 191)
`output.log('✓ Dependency removed successfully')` (singular — not pluralised by count).

### Error path (lines 192-198)
`output.error(chalk.red('✗ Failed to remove dependency:'), error.message)` then `process.exit(1)`.

## Mapping to Rust

### Core impl shape
```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```

Args (deserialise from JSON):
```rust
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RemoveDependencyArgs {
    work_unit_id: String,
    blocks: Option<String>,
    blocked_by: Option<String>,
    depends_on: Option<String>,
    relates_to: Option<String>,
}
```

Result:
```rust
#[derive(Debug, Serialize)]
struct RemoveDependencyResult { success: bool }
```

### Helpers reused from add_dependencies port
- `ensure_work_units_file` (load-or-init)
- `write_json_atomic` (atomic write)
- The `extra` map on `WorkUnit` holds `blocks`, `blockedBy`, `dependsOn`, `relatesTo` as `Vec<String>`-serialised arrays.

### Helpers ported / introduced
- `remove_from_list_field(data, id, field, value)` — read array, filter, write back or delete-when-empty.
- No cycle detection. No state-array mutations.
- iso8601_now() for `updatedAt` bump.

### CLI bridge
- `pub struct CliArgs { work_unit_id, depends_on_pos: Option<String>, blocks, blocked_by, depends_on, relates_to }`.
- Bridge reconciles positional + `--depends-on` BEFORE marshalling to JSON (mirrors TS lines 159-170).
- At-least-one guard also enforced in bridge BEFORE delegating (so the dispatcher receives only well-formed JSON; the dispatcher additionally tolerates all-empty as a no-op for forward-compat? — NO, mirror TS: reject in bridge only; dispatcher just removes whatever is present, returning `{success:true}` even when nothing matched, identical to TS).
- Stdout: `✓ Dependency removed successfully`.
- Stderr: `✗ Failed to remove dependency: <err>`.

## Test scenarios (Phase A → Gherkin)

### Rust-port dispatcher scenarios
1. Remove `blocks` edge cleans both source.blocks and target.blockedBy + deletes empty arrays.
2. Remove `blockedBy` edge cleans both source.blockedBy and target.blocks + deletes empty arrays.
3. Remove `dependsOn` cleans ONLY source array (no reverse edge touched, no status change).
4. Remove `relatesTo` cleans both source and target relatesTo arrays.
5. Removing a non-existent dependency is a silent no-op (no error, `success:true`).
6. Removing edge whose target work unit is missing: source array still updated, no error on reverse side.
7. Empty-array deletion: when the last edge is removed, the field is `delete`d (not left as `[]`).
8. Missing SOURCE work unit returns the canonical `Work unit 'X' does not exist` error and writes nothing.
9. Status of source and target is NEVER changed by removal (regression test against accidental auto-revert).
10. `updatedAt` of the SOURCE is bumped; target's `updatedAt` is NOT touched.
11. Auto-create spec/work-units.json on first run; then report missing-source.

### CLI-subcommand scenarios
1. `remove-dependency AUTH-001 AUTH-002` (positional shorthand for `--depends-on AUTH-002`) succeeds.
2. `remove-dependency AUTH-001 --depends-on AUTH-002` succeeds.
3. Positional + `--depends-on` with DIFFERENT values → exit 1 with the canonical conflict message.
4. Positional + `--depends-on` with SAME value → succeeds (no conflict).
5. No flags supplied → exit 1 with "Must specify at least one relationship" message.
6. `--blocks`, `--blocked-by`, `--relates-to` flags each route correctly.
7. Missing source unit → exit 1 with stderr `✗ Failed to remove dependency: Work unit '...' does not exist`.
8. Successful removal prints `✓ Dependency removed successfully` to stdout, exit 0.
9. Help output matches captured fixture verbatim.
