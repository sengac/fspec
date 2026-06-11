# AST Research — `add-example` (RPC-181)

## TypeScript source of truth

- `src/commands/add-example.ts` (116 LOC)
- `src/commands/add-example-help.ts` (37 LOC)
- `src/types/index.ts` — `ExampleItem` type alias of `ItemWithId`
- `src/utils/ensure-files.ts` — `ensureWorkUnitsFile` (auto-creates `spec/work-units.json`)
- `src/utils/file-manager.ts` — `fileManager.transaction()` (atomic write w/ lock)
- `src/utils/system-reminder.ts` — `wrapInSystemReminder(content)`

## TypeScript Behaviour Observations (every observable side-effect)

1. **Resolves CWD via `options.cwd || process.cwd()`**, joins `spec/work-units.json`.
2. **Calls `ensureWorkUnitsFile(cwd)`** — auto-creates the file with the canonical
   initial structure (version `0.7.1`, all 7 state arrays empty) if missing.
3. **Validates work unit exists**: `data.workUnits[id]` must be present.
   Error: `Work unit '<id>' does not exist`.
4. **Validates work unit status is `specifying`**.
   Error: `Can only add examples during discovery/specification phase. <id> is in '<state>' state.`
5. **Initializes `workUnit.examples = []` when undefined**.
6. **Initializes `workUnit.nextExampleId = 0` when undefined** (backward-compat).
7. **Creates an `ExampleItem`** with stable id from `nextExampleId++`, the supplied
   `text`, `deleted: false`, and `createdAt: new Date().toISOString()`.
8. **Pushes to `workUnit.examples`**.
9. **Bumps `workUnit.updatedAt = new Date().toISOString()`**.
10. **Atomic write via `fileManager.transaction(workUnitsFile, async fileData => Object.assign(fileData, data))`**.
11. **Builds a `systemReminder`** wrapped in `<system-reminder>` tags. The body
    references the user-story `role` (or `"the user"` fallback) and the example
    text. The CLI prints this AFTER the success line.
12. **CLI returns success line `✓ Example added successfully`** followed by a
    blank line and the system reminder.
13. **CLI error path**: `process.exit(1)` with stderr `✗ Failed to add example: <message>`.

## Two front doors

- Shell argv:  `fspec add-example <workUnitId> <example>` (Commander.js, 2 positional args)
- LLM tool call: `{"command":"add-example","args":{"workUnitId":"...","example":"..."}}`

## Help config (TS canon)

```
name        : add-example
description : Add a concrete example to a work unit during Example Mapping
usage       : fspec add-example <workUnitId> <example>
whenToUse   : Use during specifying phase to capture concrete examples that illustrate rules and will become test scenarios.
arguments   : workUnitId (required), example (required)
examples    : `fspec add-example AUTH-001 "Login with user@example.com and correct password"` → ✓ Example added successfully
related     : add-rule, add-question, generate-scenarios, remove-example
```

## Rust port plan

- `codelet/fspec-core/src/commands/add_example.rs`:
  - Args (`#[serde(rename_all = "camelCase")]`): `work_unit_id: String, example: String`.
  - Use `ensure_work_units_file(project_root)?` → `WorkUnitsData`.
  - Validate exists / state == `specifying`.
  - Mutate `extra` fields (`examples`, `nextExampleId`) since they live in the
    flattened `serde_json::Map<String, Value>` on `WorkUnit`.
  - Build `ExampleItem` as `serde_json::Map` w/ field insertion order
    `id, text, deleted, createdAt` to match TS object-literal order.
  - Bump `updated_at`.
  - Write via `write_json_atomic`.
  - Render text result with success line + blank line + system reminder.
- `codelet/fspec-core/src/help/configs/add_example.rs` — `CONFIG` mirrors TS help.
- `codelet/fspec/src/add_example.rs` — bridge w/ `CliArgs { work_unit_id, example }`.

## Open questions
None — TS parity is complete.
