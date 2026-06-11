# AST Research — restore-question (RPC-290)

## TS Source: `src/commands/restore-question.ts`

### Signature
- `restoreQuestion({workUnitId, index, cwd?}) -> Promise<RestoreQuestionResult>`
- Result: `{ success: boolean, restoredQuestion: string, activeCount: number, message?: string }`

### Behaviour (observed from TS source)
1. Read `spec/work-units.json` via `ensureWorkUnitsFile(cwd)` (auto-creates).
2. Throw if `data.workUnits[workUnitId]` is missing → `Work unit '<id>' does not exist`.
3. Validate `workUnit.status === 'specifying'` else throw
   `Can only restore questions during discovery/specification phase. <id> is in '<status>' state.`
4. Validate `workUnit.questions` array exists and non-empty else throw
   `Work unit <id> has no questions`.
5. Lookup `question` by `q.id === options.index` (STABLE id, NOT positional).
   Throw `Question with ID <index> not found` if missing.
6. **Idempotent path**: if `!question.deleted` (already active), return
   `{ success: true, restoredQuestion: question.text, activeCount: <non-deleted-count>, message: 'Item ID <index> already active' }`
   WITHOUT mutating disk.
7. Restore: `question.deleted = false; delete question.deletedAt;`
8. Update `workUnit.updatedAt = new Date().toISOString();`  
   (NOTE: does NOT update `data.meta.lastUpdated` — contrast with restore-architecture-note which does.)
9. Persist with `fileManager.transaction(workUnitsFile, …)` — atomic write.
10. Return success with `restoredQuestion`, `activeCount` (count of `!q.deleted`).

### CLI Surface (Commander.js)
- Subcommand: `restore-question`
- Positional `<workUnitId>` (required)
- Positional `<index>` (required, parsed via `parseInt(index, 10)`)
- On success: `chalk.green('✓ Restored question: "<text>"')`, then if `result.message` also prints `  <message>` (indented).
- On error: `output.error('✗ Failed to restore question:', error.message)`, `process.exit(1)`.

NB the success output uses the prefix `✓ Restored question: "<text>"` — NOT the prefix in the help doc which mentions "✓ Restored question from AUTH-001". Help is divergent; **TS CLI output is canon** for the binary's runtime stdout. Help fixture parity test asserts byte-equality with the captured TS help text.

### Help Source: `src/commands/restore-question-help.ts`
- Name: `restore-question`
- Description: `Restore soft-deleted question by stable ID (undeletes item and clears deletedAt timestamp)`
- Usage: `fspec restore-question <workUnitId> <index>`
- Arguments: `workUnitId`, `index`
- Options: `--ids <ids>` (Restore multiple questions… comma-separated)
  - NOTE: The TS implementation file does NOT wire `--ids` into the Commander action. The help advertises it but the actual TS CLI ignores it. Out of scope for first port — we mirror Commander.js's surface (positional-only), and the `--ids` flag appears in help fixture only.
- Examples / Notes / aiGuidance / relatedCommands — pasted into help config.

## Rust Mirror Strategy

### Files (worker-owned)
- `codelet/fspec-core/src/commands/restore_question.rs` — single source of truth (replace stub).
- `codelet/fspec-core/src/help/configs/restore_question.rs` — help config mirroring `restore-question-help.ts`.
- `codelet/fspec/src/restore_question.rs` — CLI bridge (JSON marshal only).
- `codelet/fspec-core/tests/restore_question.rs` — dispatcher tests.
- `codelet/fspec/tests/cli_restore_question.rs` — CLI integration tests.
- `codelet/fspec/tests/fixtures/help/restore-question.txt` — byte-exact `--help` capture.

### Shared (supervisor-owned)
- `codelet/fspec-core/src/canonical.rs::PORTED_COMMANDS` — add `restore-question`.
- `codelet/fspec-core/src/dispatch.rs::run_ported` — add match arm; remove from `run_stub`.
- `codelet/fspec-core/src/commands/mod.rs` — module already registered (stub).
- `codelet/fspec-core/src/help/configs/mod.rs` — register new help config.
- `codelet/fspec/src/main.rs` — add `Mode::RestoreQuestion` variant, `forward!` arm, intercept arm, `mod restore_question;`.

### Shared infrastructure REUSED (no new helpers needed)
- `crate::io::ensure::ensure_work_units_file` — load/auto-create.
- `crate::io::locked_file::write_json_atomic` — atomic write.
- `crate::io::time::iso8601_now` — millisecond ISO-8601 timestamps.

### Args struct (`#[serde(rename_all = "camelCase")]`)
```rust
struct RestoreQuestionArgs {
    work_unit_id: String,
    index: u64,
}
```

### Result struct (serialized to JSON for dispatcher; CLI bridge wraps with chalk)
```rust
struct RestoreQuestionResult {
    success: bool,
    #[serde(rename = "restoredQuestion")] restored_question: String,
    #[serde(rename = "activeCount")] active_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")] message: Option<String>,
}
```

### CLI Output (render fn in dispatcher)
Two output modes: dispatcher returns the canonical CLI text (no chalk colours):
- Default: `✓ Restored question: "<text>"\n` (+ trailing `  Item ID <id> already active\n` when idempotent).
- Errors raise `FspecCoreError::InvalidArgs { command: "restore-question", reason: <message> }`.

CLI bridge `restore_question.rs` only marshals JSON and prints whatever the core returns; error case prints `Error: <err>` to stderr (mirroring TS `output.error('✗ Failed to restore question:', error.message)` — semantically equivalent for piped output).

## Reference Pattern
- `codelet/fspec-core/src/commands/remove_question.rs` (RPC-278) — closest mirror, also operates on `questions` array via stable id, also has idempotent-already-deleted path. Restore is the inverse mutation: clear `deleted` flag + delete `deletedAt`.
