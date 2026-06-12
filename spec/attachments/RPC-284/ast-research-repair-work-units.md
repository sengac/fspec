# AST Research — repair-work-units (RPC-284)

TS source of truth: `src/commands/repair-work-units.ts` (152 LOC)
Help: `src/commands/repair-work-units-help.ts`
No dedicated TS unit-test file found (grep `repair-work-units` only matches src + help).

## Public surface (TS)

```
repairWorkUnits({ cwd? })  -> { success: boolean, repairs: string[], repaired: number }
```

Commander.js CLI:
- `--dry-run` ("Show what would be repaired without making changes")

**TS BUG (lines 128-152, registrar):** The registrar passes `dryRun` into
`repairWorkUnits({ dryRun })` but the impl signature only accepts `{ cwd? }` and IGNORES
dryRun entirely — it ALWAYS writes. The registrar then reads `result.details` (undefined,
function returns `repairs`/`repaired`), so the per-repair lines are NEVER printed via CLI.
Output is just `✓ Repaired <n> issues`. The `--dry-run` flag is accepted by Commander but
has NO effect (still writes). Rust port MUST preserve this behaviour for byte parity:
dry-run is a no-op flag; the file is always written.

## Algorithm (TS lines 19-126)

1. `cwd = options.cwd || process.cwd()`; `repairs: string[] = []`.
2. Load `ensureWorkUnitsFile(cwd)` (auto-create). Path `spec/work-units.json`.
3. **Rebuild states from scratch** — new empty arrays for all 7 states
   (backlog, specifying, testing, implementing, validating, done, blocked).
4. For each `[id, workUnit]` in `Object.entries(workUnits)` (insertion order):
   - `status = workUnit.status`.
   - if `newStates[status]` exists: push `id` into `newStates[status]`.
   - Then check OLD states: for each `[stateName, ids]` in `Object.entries(oldStates)`,
     if `stateName !== status && ids.includes(id)` → push repair message
     `Moved <id> from <stateName> to <status>`.
5. `data.states = newStates`.
6. **Bidirectional dependency repair** — for each `[id, workUnit]`:
   - **blocks**: for each `targetId` in `workUnit.blocks` (if present):
     if `workUnits[targetId]` exists: ensure `target.blockedBy` array exists; if
     `!target.blockedBy.includes(id)` → push `id` and repair message
     `Repaired bidirectional link: <id> blocks <targetId>`.
   - **blockedBy**: for each `targetId` in `workUnit.blockedBy`:
     if `workUnits[targetId]` exists: ensure `target.blocks` array; if
     `!target.blocks.includes(id)` → push `id` and repair message
     `Repaired bidirectional link: <targetId> blocks <id>`.
   - **relatesTo**: for each `targetId` in `workUnit.relatesTo`:
     if `workUnits[targetId]` exists: ensure `target.relatesTo` array; if
     `!target.relatesTo.includes(id)` → push `id` and repair message
     `Repaired bidirectional link: <id> relates to <targetId>`.
7. Atomic write via `fileManager.transaction`.
8. return `{ success: true, repairs, repaired: repairs.length }`.

## Notes for Rust port

- `blocks` / `blockedBy` / `relatesTo` live in WorkUnit `extra` map (not typed fields).
  Read as `extra.get("blocks").and_then(Value::as_array)`. Mutate target's array in extra.
- IMPORTANT: mutating one work unit's extra (target.blockedBy) while iterating over the
  IndexMap requires care in Rust — clone the id list / collect mutation plan first, then
  apply, to satisfy the borrow checker. Order of repair messages must match TS (outer loop
  is insertion order; inner loops are array order; states-moves computed during the
  states rebuild loop, dependency repairs after, in a SECOND pass).
- `states` array order within each bucket = insertion order of workUnits (TS push order).
- newStates buckets created in fixed key order backlog→specifying→...→blocked. The Rust
  `WorkUnitStates` struct already has this fixed field order.
- dispatcher result is the full `{ success, repairs, repaired }` JSON (pretty). CLI prints
  only `✓ Repaired <repaired> issues` (and the buggy details loop that never fires).
- write_json_atomic always (dry-run ignored — see TS BUG above).
- updatedAt is NOT bumped per-unit in this command.

## AST findings

- `grep "export async function repairWorkUnits"` — single exported impl fn.
- `grep "registerRepairWorkUnitsCommand"` — single Commander registrar.
- Data-integrity error in prioritize-work-unit references `fspec repair-work-units` (sibling).
- Stub: `codelet/fspec-core/src/commands/repair_work_units.rs` returns NotYetPorted (RPC-284).
- Dispatcher arm: `dispatch.rs:570 "repair-work-units" => commands::repair_work_units::run(args_json).await` (OLD single-arg signature — supervisor must rewire to `(args_json, project_root)`).
