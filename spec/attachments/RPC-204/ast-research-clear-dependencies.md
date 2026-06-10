# AST Research — `clear-dependencies` (RPC-204)

**TS source**: `src/commands/clear-dependencies.ts` (124 LOC)
**Goal**: Port to Rust at `codelet/fspec-core/src/commands/clear_dependencies.rs` + CLI bridge.

---

## Signature

```ts
interface ClearDependenciesOptions {
  workUnitId: string;
  confirm?: boolean;
  cwd?: string;
}
interface ClearDependenciesResult { success: boolean; }
export async function clearDependencies(options): Promise<ClearDependenciesResult>
```

## Behaviour summary (lines 22-99)

1. **--confirm guard** (lines 24-28): if `options.confirm` is falsy, throw
   `Error('Must confirm clearing all dependencies with --confirm flag')`.
   No work-units file is loaded yet.
2. **Load** `spec/work-units.json` via `ensureWorkUnitsFile(cwd)`.
3. **Source-exists** (lines 33-35): throw `Error("Work unit '<id>' does not exist")` if absent.
4. **blocks branch (bidirectional)** (lines 40-52):
   - For each target id in `workUnit.blocks`:
     - If `data.workUnits[targetId]?.blockedBy` exists, filter source id out;
       if result empty → `delete blockedBy`.
   - `delete workUnit.blocks`.
5. **blockedBy branch (bidirectional)** (lines 55-67):
   - Mirror of (4) — for each target in `workUnit.blockedBy`, filter source id from
     target's `blocks`, delete if empty. Then `delete workUnit.blockedBy`.
6. **dependsOn branch (UNIDIRECTIONAL)** (lines 70-72):
   - Just `delete workUnit.dependsOn`. No reverse-edge cleanup.
7. **relatesTo branch (symmetric bidirectional)** (lines 75-87):
   - For each target in `workUnit.relatesTo`, filter source id from
     `data.workUnits[targetId]?.relatesTo`; delete if empty.
   - `delete workUnit.relatesTo`.
8. **Bump updatedAt** on source (line 89). No state-array mutations, no status changes.
9. **Atomic write** via `fileManager.transaction(workUnitsFile, ...)` (lines 92-94).
10. Return `{ success: true }`.

### Critical divergences vs `add-*` commands

* NO cycle detection (removal cannot create cycles).
* NO status transitions — a unit blocked exclusively by an edge being cleared
  STAYS `status=blocked`. Matches `remove-dependency` semantics.
* NO `states.<status>` array mutations.
* Reverse-edge cleanup is **silently skipped** when the target work unit does
  not exist (TS guard `data.workUnits[targetId]?.blockedBy`).

## CLI surface (Commander, lines 101-123)

```
fspec clear-dependencies <workUnitId>
  --confirm    Confirm clearing all dependencies
```

Action handler:
- Success: `output.log(chalk.green(\`✓ All dependencies cleared from ${workUnitId}\`))`
- Failure: `output.error(chalk.red('✗ Failed to clear dependencies:'), error.message)`
  then `process.exit(1)`.

## Rust port plan

| Concern                | Approach                                                                                         |
|------------------------|--------------------------------------------------------------------------------------------------|
| Args struct            | `ClearDependenciesArgs { work_unit_id: String, confirm: bool }`                                  |
| Result struct          | `#[derive(Serialize)] ClearDependenciesResult { success: bool }`                                 |
| --confirm enforcement  | core checks `args.confirm == true` → else `InvalidArgs` "Must confirm clearing all dependencies with --confirm flag" |
| Persistence            | `ensure_work_units_file` → in-memory mutations → SINGLE `write_json_atomic` at end               |
| Bidirectional cleanup  | `clear_field_with_reverse(data, source_id, field, reverse_field)` helper                         |
| Unidirectional cleanup | direct `wu.extra.remove("dependsOn")`                                                             |
| Field iteration order  | `blocks → blockedBy → dependsOn → relatesTo` (preserves TS branch order)                         |
| updatedAt              | only source bumped (`iso8601_now()`)                                                              |
| CLI bridge             | `CliArgs { work_unit_id: String, confirm: bool }` → `{workUnitId, confirm}` JSON                 |

## File layout

- `codelet/fspec-core/src/commands/clear_dependencies.rs` (~180 LOC)
- `codelet/fspec-core/src/help/configs/clear_dependencies.rs` (help config)
- `codelet/fspec/src/clear_dependencies.rs` (~80 LOC CLI bridge)
- `codelet/fspec-core/tests/clear_dependencies.rs` (dispatcher tests)
- `codelet/fspec/tests/cli_clear_dependencies.rs` (CLI shell tests)
- `codelet/fspec/tests/fixtures/help/clear-dependencies.txt` (help fixture)

## Validation observations

- All-empty cleanup (work unit has no deps) is a no-op success.
- Self-references in arrays (corrupt state) — TS filters them too; we will not
  add explicit validation, just mirror behaviour.
- Missing target in reverse-edge cleanup → silently skipped, NO error.
