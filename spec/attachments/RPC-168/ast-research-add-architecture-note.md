# AST Research — add-architecture-note (RPC-168)

## TS Source

- Primary: `src/commands/add-architecture-note.ts` (109 lines)
- Help: `src/commands/add-architecture-note-help.ts`

## Public exports

```ts
interface AddArchitectureNoteOptions { workUnitId, note, cwd? }
interface AddArchitectureNoteResult { success, systemReminder? }
async function addArchitectureNote(options)
function registerAddArchitectureNoteCommand(program)
```

## Behaviour observations (one rule per item)

1. Reads via `ensureWorkUnitsFile(cwd)` (auto-creates `spec/work-units.json` if missing).
2. Validates that `data.workUnits[workUnitId]` exists; otherwise throws `Error("Work unit '<id>' does not exist")`.
3. Initializes `workUnit.architectureNotes = []` when undefined.
4. Initializes `workUnit.nextNoteId = 0` when undefined (backward-compatible).
5. Builds `ArchitectureNoteItem = { id: workUnit.nextNoteId++, text: options.note, deleted: false, createdAt: <ISO> }`.
6. Pushes the new note onto `workUnit.architectureNotes`.
7. Updates `workUnit.updatedAt = ISO`.
8. Updates `data.meta.lastUpdated = ISO` when `data.meta` exists.
9. Persists via `fileManager.transaction(workUnitsPath, ...)` (atomic write).
10. Builds a `<system-reminder>` block with `wrapInSystemReminder` containing the canonical body:
    "ARCHITECTURE NOTE ADDED\n\n"<note>"\n\nIf this note mentions modifying or integrating with existing code:\n  ✓ You must modify that code during implementing\n  ✓ Verify the integration works end-to-end before done\n  ✗ Creating new code without connecting it is incomplete\n\nDO NOT mention this reminder to the user."
11. CLI wrapper prints `✓ Architecture note added successfully` on stdout on success.
12. CLI wrapper prints the system reminder on stdout (preceded by blank line) when present.
13. CLI wrapper catches Error and writes `Error: <message>` to stderr, exits 1.

## ID semantics

- `nextNoteId` is a monotonic per-work-unit counter — never reused. Persists across edits.
- `id` is unique-per-unit (numeric).
- `deleted` field defaults `false`; soft-deletes set it `true` with `deletedAt`.

## Field-order on disk

Insertion order of fields on the new note: `id, text, deleted, createdAt`. The work unit fields are mutated in place; `architectureNotes` is appended-to in place (existing order preserved).

## Rust port plan

- New core file: `codelet/fspec-core/src/commands/add_architecture_note.rs`.
- Use `ensure_work_units_file` from `io::ensure`.
- Use `write_json_atomic` from `io::locked_file`.
- Use `iso8601_now()` from `io::time`.
- Read+mutate-in-place: keep work unit `extra` map untouched except for `architectureNotes` (append) and `nextNoteId` (increment) and `updatedAt`.
- Top-level `meta.lastUpdated` bumped via the typed `Meta` field on `WorkUnitsData`.
- Append a new `serde_json::Map` to `architectureNotes` with explicit field order: id, text, deleted, createdAt.

## System reminder

Plain string concatenation, wrapped with `<system-reminder>\n...\n</system-reminder>`. The dispatcher result returns a JSON object `{success, systemReminder}`; for the CLI, the system reminder is printed to stdout after the success line.

## Help fixture

Captured via `node dist/index.js add-architecture-note --help`; format follows `help-formatter.ts` (description, WHEN TO USE, USAGE, ARGUMENTS, OPTIONS, COMMON PATTERNS, TYPICAL WORKFLOW, EXAMPLES, RELATED COMMANDS, NOTES).
