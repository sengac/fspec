# AST Research — restore-architecture-note (RPC-287)

## TS Source: `src/commands/restore-architecture-note.ts`

### Signature
- `restoreArchitectureNote({workUnitId, index, cwd?}) -> Promise<RestoreArchitectureNoteResult>`
- Result: `{ success: boolean, restoredNote: string, activeCount: number, message?: string }`

### Behaviour (observed from TS source)
1. Read `spec/work-units.json` via `ensureWorkUnitsFile(cwd)` (auto-creates).
2. Throw if `data.workUnits[workUnitId]` is missing → `Work unit '<id>' does not exist`.
3. **NOTE**: Unlike restore-question/example/rule, this command does NOT validate
   `workUnit.status === 'specifying'`. The TS source has no status gate for
   architecture-note restoration.
4. Validate `workUnit.architectureNotes` array exists and non-empty else throw
   `Work unit '<id>' has no architecture notes`.
5. Lookup `note` by `n.id === options.index` (STABLE id, NOT positional).
   Throw `Architecture note with ID <index> not found` if missing.
6. **Idempotent path**: if `!note.deleted` (already active), return
   `{ success: true, restoredNote: note.text, activeCount: <non-deleted-count>, message: 'Item ID <index> already active' }`
   WITHOUT mutating disk.
7. Restore: `note.deleted = false; delete note.deletedAt;`
8. Update `workUnit.updatedAt = new Date().toISOString();`
9. ALSO updates `data.meta.lastUpdated = new Date().toISOString();` (contrast with restore-question).
10. Persist with `fileManager.transaction(workUnitsPath, …)` — atomic write.
11. Return success with `restoredNote`, `activeCount` (count of `!n.deleted`).

### CLI Surface (Commander.js)
- Subcommand: `restore-architecture-note`
- Positional `<workUnitId>` (required)
- Positional `<index>` (required, parsed via `parseInt` Commander coercion)
- On success: `output.log('✓ Architecture note restored successfully')`, then if `result.message` also prints `  <message>` (indented).
- On error: `output.error('Error:', errorMessage)`, `process.exit(1)`.

NB the success line is the static `✓ Architecture note restored successfully` (no embedded text). The result `restoredNote` is in the dispatcher JSON payload but NOT printed by the TS CLI.

### Help Source: `src/commands/restore-architecture-note-help.ts`
- Name: `restore-architecture-note`
- Description: `Restore soft-deleted architecture note by stable ID (undeletes item and clears deletedAt timestamp)`
- Usage: `fspec restore-architecture-note <workUnitId> <index>`
- Arguments: `workUnitId`, `index`
- Options: `--ids <ids>` (advertised in help but NOT wired into TS Commander action — help-canon-only flag, mirror in fixture but the binary's clap surface omits it).
- Examples / Notes / aiGuidance / relatedCommands — pasted into help config.

## Rust Mirror Strategy

### Files (worker-owned)
- `codelet/fspec-core/src/commands/restore_architecture_note.rs` — replace stub.
- `codelet/fspec-core/src/help/configs/restore_architecture_note.rs` — new help config mirroring `restore-architecture-note-help.ts`.
- `codelet/fspec/src/restore_architecture_note.rs` — CLI bridge (JSON marshal only).
- `codelet/fspec-core/tests/restore_architecture_note.rs` — dispatcher tests.
- `codelet/fspec/tests/cli_restore_architecture_note.rs` — CLI integration tests.
- `codelet/fspec/tests/fixtures/help/restore-architecture-note.txt` — byte-exact `--help` capture.

### Shared (supervisor-owned)
- `codelet/fspec-core/src/canonical.rs::PORTED_COMMANDS` — add `restore-architecture-note`.
- `codelet/fspec-core/src/dispatch.rs::run_ported` — add match arm; remove from `run_stub`.
- `codelet/fspec-core/src/help/configs/mod.rs` — register new help config (currently the stub registration may already exist; check).
- `codelet/fspec/src/main.rs` — add `Mode::RestoreArchitectureNote` variant, `forward!` arm, intercept arm, `mod restore_architecture_note;`.

### Shared infrastructure REUSED (no new helpers needed)
- `crate::io::ensure::ensure_work_units_file` — load/auto-create.
- `crate::io::locked_file::write_json_atomic` — atomic write.
- `crate::io::time::iso8601_now` — millisecond ISO-8601 timestamps.

### Args struct (`#[serde(rename_all = "camelCase")]`)
```rust
struct RestoreArchitectureNoteArgs {
    work_unit_id: String,
    index: u64,
}
```

### Result struct (serialized for dispatcher)
```rust
struct RestoreArchitectureNoteResult {
    success: bool,
    #[serde(rename = "restoredNote")] restored_note: String,
    #[serde(rename = "activeCount")] active_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")] message: Option<String>,
}
```

### Dispatcher output (text)
- Default: `✓ Architecture note restored successfully\n`
- Idempotent: append `  Item ID <id> already active\n`
- Errors → `FspecCoreError::InvalidArgs { command: "restore-architecture-note", reason }`.

## Reference Pattern
- `codelet/fspec-core/src/commands/remove_architecture_note.rs` (RPC-267) — closest mirror, operates on `architectureNotes` array, soft-deletes by stable id, has idempotent-already-deleted path. Restore inverts the mutation.
- Key divergence from sibling restore commands: **no status gate** (any status allowed).
