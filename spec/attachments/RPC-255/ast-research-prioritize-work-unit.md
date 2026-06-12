# AST Research — prioritize-work-unit (RPC-255)

TS source of truth: `src/commands/prioritize-work-unit.ts` (173 LOC)
Help: `src/commands/prioritize-work-unit-help.ts`
Existing TS tests: `src/commands/__tests__/prioritize-work-unit-data-integrity.test.ts`,
`src/commands/__tests__/prioritize-work-unit-1-based-indexing.test.ts`

## Public surface (TS)

```
prioritizeWorkUnit({ workUnitId, position?: 'top'|'bottom'|number, before?, after?, cwd? })
  -> { success: boolean }
```

Commander.js CLI:
- positional `<workUnitId>` (required)
- `--position <position>` ("top" | "bottom" | numeric — parsed via parseInt base 10)
- `--before <workUnitId>`
- `--after <workUnitId>`

CLI position parsing (TS lines 147-154):
- `"top"` → 'top'
- `"bottom"` → 'bottom'
- any other truthy → `parseInt(position, 10)` (number)
- absent → undefined

## Algorithm (TS lines 24-130)

1. `cwd = options.cwd || process.cwd()`.
2. Load `ensureWorkUnitsFile(cwd)` (auto-create). File path `spec/work-units.json`.
3. **Existence**: if `!workUnits[workUnitId]` → throw `Work unit '<id>' does not exist`.
4. Capture `workUnit = workUnits[workUnitId]`.
5. **Done guard**: if `workUnit.status === 'done'` → throw the long message:
   `Cannot prioritize work units in done column. Done items are ordered by completion time and cannot be manually reordered. Only backlog, specifying, testing, implementing, validating, blocked can be prioritized.`
6. **Target existence**: if `before` set and `!workUnits[before]` → throw `Work unit '<before>' does not exist`. Same for `after`.
7. **Cross-column guard**: if `before` set and `before.status !== workUnit.status` → throw
   `Cannot prioritize across columns. <id> (<status>) and <before> (<beforeStatus>) are in different columns.`
   Same for `after`.
8. `currentStatus = workUnit.status`.
9. **Data-integrity (self)**: if `!states[currentStatus].includes(workUnitId)` → throw
   `Data integrity error: Work unit <id> has status '<status>' but is not in states.<status> array. Run 'fspec repair-work-units' to fix data corruption.`
10. `column = states[currentStatus].filter(id => id !== workUnitId)` (removes self, safe vs duplicates).
11. Determine `newIndex` (default 0):
    - position 'top' → 0
    - position 'bottom' → column.length
    - position number → `position - 1`; if `< 0` throw
      `Invalid position: <position>. Position must be >= 1 (1-based index)`. (Positions beyond length are allowed — splice clamps to end.)
    - before → `newIndex = column.indexOf(before)`; if `-1` throw the same Data-integrity message for `before`.
    - after → `newIndex = column.indexOf(after)`; if `-1` throw Data-integrity message for `after`; else `newIndex += 1`.
12. `column.splice(newIndex, 0, workUnitId)`.
13. `states[currentStatus] = column`.
14. Atomic write via `fileManager.transaction` (Object.assign whole data).
15. return `{ success: true }`.

## Notes for Rust port

- Position can be 'top'/'bottom'/integer/absent. In JSON from dispatcher, `position` may
  arrive as a string ("top"/"bottom") OR a number. Use `serde_json::Value` or an untagged
  enum to accept both. CLI bridge parses argv string → marshals 'top'/'bottom' as string,
  numeric as JSON number.
- Insertion order of `workUnits` IndexMap is NOT reordered — only `states.<status>` array changes.
- `position` number 0 and negatives both rejected (newIndex < 0). Note position 0 → newIndex -1 → reject.
- splice with newIndex beyond column.length inserts at end (Vec::insert would panic — must clamp `newIndex.min(column.len())`).
- All errors map to FspecCoreError::InvalidArgs with the verbatim TS reason strings.
- No updatedAt bump in this command (unlike add-rule). Only states array mutated.
- write_json_atomic on the whole WorkUnitsData.

## AST findings

- `grep "export async function prioritizeWorkUnit"` — single exported impl fn.
- `grep "registerPrioritizeWorkUnitCommand"` — single Commander registrar.
- No other call sites of `prioritizeWorkUnit` outside its own tests.
- Stub: `codelet/fspec-core/src/commands/prioritize_work_unit.rs` returns NotYetPorted (RPC-255).
- Dispatcher arm: `dispatch.rs:522 "prioritize-work-unit" => commands::prioritize_work_unit::run(args_json).await` (OLD single-arg signature — supervisor must rewire to `(args_json, project_root)`).
