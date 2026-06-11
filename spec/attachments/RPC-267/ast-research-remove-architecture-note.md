# AST Research — remove-architecture-note (RPC-267)

## TS Source

- Primary: `src/commands/remove-architecture-note.ts` (108 lines)
- Help: `src/commands/remove-architecture-note-help.ts`

## Public exports

```ts
interface RemoveArchitectureNoteOptions { workUnitId, index, cwd? }
interface RemoveArchitectureNoteResult { success, removedNote, remainingCount, message? }
async function removeArchitectureNote(options)
function registerRemoveArchitectureNoteCommand(program)
```

## Behaviour observations

1. Reads via `ensureWorkUnitsFile(cwd)` (auto-creates `spec/work-units.json` if missing).
2. Errors with `Work unit '<id>' does not exist` when the work unit is missing.
3. Errors with `Work unit '<id>' has no architecture notes` when `architectureNotes` is missing OR empty.
4. The `index` argument is treated as the STABLE ID (per the comment "index is now treated as ID for stable indices"), NOT an array position.
5. Looks up the note via `architectureNotes.find(n => n.id === options.index)`.
6. Errors with `Architecture note with ID <index> not found` when no note matches.
7. If the matched note is already `deleted=true`, returns idempotent success: `{success: true, removedNote: note.text, remainingCount: <active count>, message: "Item ID <index> already deleted"}`. No disk write.
8. Otherwise sets `note.deleted = true`, `note.deletedAt = <ISO>` (soft delete).
9. Captures `removedNote = note.text` BEFORE returning.
10. Updates `workUnit.updatedAt = ISO`.
11. Updates `data.meta.lastUpdated = ISO` when present.
12. Persists via `fileManager.transaction(workUnitsPath, ...)` (atomic write).
13. Returns `{success: true, removedNote, remainingCount: <active count>}`.
14. CLI wrapper prints `✓ Architecture note removed successfully` on stdout.
15. When `result.message` is set (idempotent path), prints `  <message>` indented on a second line.
16. CLI wrapper catches Error → writes `Error: <msg>` to stderr, exits 1.
17. The CLI `<index>` arg is parsed with `parseInt` (Commander's value-coercion) — for the Rust port, take `u32` directly through clap and serialize as integer.

## Rust port plan

- New core file: `codelet/fspec-core/src/commands/remove_architecture_note.rs`.
- Use `ensure_work_units_file` and `write_json_atomic`.
- Use `iso8601_now()` for timestamps.
- Iterate `architectureNotes` (which lives in `extra` map as `Value::Array`) looking for the object whose `id` field equals the requested numeric index. Mutate in place: set `deleted = true`, `deletedAt = <iso>`.
- For idempotent already-deleted path, return without writing.
- Return JSON-encoded `{success, removedNote, remainingCount, message?}` from the core; the CLI bridge formats stdout.

## Help fixture

Captured via `node dist/index.js remove-architecture-note --help`; standard CommandHelpConfig shape.
