# AST Research — `remove-example` (RPC-273)

## TypeScript source of truth

- `src/commands/remove-example.ts` (108 LOC)
- `src/commands/remove-example-help.ts` (29 LOC)
- `src/types/index.ts` — `ExampleItem` type alias of `ItemWithId`
- `src/utils/ensure-files.ts` — `ensureWorkUnitsFile`
- `src/utils/file-manager.ts` — `fileManager.transaction()`

## TypeScript Behaviour Observations

1. Resolves CWD via `options.cwd || process.cwd()`, joins `spec/work-units.json`.
2. Calls `ensureWorkUnitsFile(cwd)` — auto-creates if missing.
3. Validates work unit exists: `Work unit '<id>' does not exist`.
4. Validates status == `specifying`:
   `Can only remove examples during discovery/specification phase. <id> is in '<state>' state.`
5. Validates examples array exists AND length > 0: `Work unit <id> has no examples`.
6. Treats `index` as STABLE ID — `workUnit.examples.find(e => e.id === options.index)`.
7. If not found: `Example with ID <index> not found`.
8. **Idempotent re-delete**: if `example.deleted === true`, return success with
   `message: "Item ID <index> already deleted"` and DOES NOT touch disk (no write).
   `remainingCount` excludes deleted.
9. **Soft-delete**: set `deleted = true`, `deletedAt = ISO timestamp`.
10. Captures `removedExample = example.text` BEFORE the set is observable on disk.
11. Bumps `workUnit.updatedAt = ISO timestamp`.
12. Atomic write via `fileManager.transaction`.
13. CLI prints `✓ Removed example: "<removedExample text>"` on success.
14. CLI exits 1 with `✗ Failed to remove example: <message>` on error.

## CLI registration (Commander.js)

```
fspec remove-example <workUnitId> <index>
  - index is parsed via parseInt(index, 10)
```

## Help canon

```
name        : remove-example
description : Remove an example from Example Mapping by index
usage       : fspec remove-example <workUnitId> <index>
arguments   : workUnitId (required), index (required)
examples    : `fspec remove-example AUTH-001 2` → ✓ Removed example from AUTH-001
related     : add-example, show-work-unit
```

## Rust port plan

- Args (`#[serde(rename_all = "camelCase")]`):
  `work_unit_id: String`, `index: u64`.
  TS uses `parseInt` so we accept JSON `number` for the dispatcher path and
  parse positional string → u64 in the bridge.
- ExampleItem lives in WorkUnit.extra["examples"] — a JSON array of objects.
  We must locate by id field (NOT array index).
- Idempotent path returns success WITHOUT writing.
- Render: `✓ Removed example: "<text>"`.

## Open questions
None.
