# AST Research — `compact-work-unit` (RPC-206)

TS source of truth: `src/commands/compact-work-unit.ts` (+ `compact-work-unit-help.ts`).
Rust target: `codelet/fspec-core/src/commands/compact_work_unit.rs` (currently `NotYetPorted` stub).

## TS behaviour (src/commands/compact-work-unit.ts)

Options (`CompactWorkUnitOptions`):
- `workUnitId: string` (required positional `<workUnitId>`)
- `force?: boolean` (`--force`) — behaviourally significant
- `cwd?: string` (defaults to `process.cwd()`)

Result (`CompactWorkUnitResult`):
```
{ success: true,
  removedCounts:   { rules, examples, questions, architectureNotes },
  remainingCounts: { rules, examples, questions, architectureNotes },
  warning?: string }
```

### compactArray<T extends {id:number; deleted?:boolean}>(items)
- undefined/empty → `{ filtered: [], removed: 0, remaining: 0 }`.
- `filtered = items.filter(i => !i.deleted)` (drops soft-deleted).
- Renumber: `filtered.forEach((item, idx) => item.id = idx)` — sequential IDs from 0.
- removed = originalLength - filtered.length; remaining = filtered.length.

### Algorithm (in order)
1. `workUnitsFile = join(cwd, 'spec/work-units.json')`.
2. `ensureWorkUnitsFile(cwd)` — load-or-init (auto-creates). → Rust `ensure_work_units_file`.
3. **Existence check**: if `!data.workUnits[id]` → throw `Work unit '<id>' does not exist`.
4. **Force gate**: if `workUnit.status !== 'done'`:
   - if `!force` → throw
     `Cannot compact work unit in '<status>' status. Use --force to confirm compaction during active development.`
   - else set `warning = "Compacting during '<status>' status permanently removes deleted items. Use with caution."`
5. Compact the FOUR arrays in order: `rules`, `examples`, `questions`, `architectureNotes`.
   Each: replace array with filtered+renumbered, record removed/remaining counts.
6. Reset counters: `nextRuleId = rules?.length ?? 0`, `nextExampleId`, `nextQuestionId`, `nextNoteId`.
7. `workUnit.updatedAt = new Date().toISOString()`.
8. `if (data.meta) data.meta.lastUpdated = new Date().toISOString()`.
9. Atomic write: `fileManager.transaction(file, fileData => Object.assign(fileData, data))`.
   → Rust single `write_json_atomic`.
10. Return result.

### CLI registration (action) — NOTE: actual CLI output, NOT the help-doc examples
- `totalRemoved = sum of removedCounts`.
- if `totalRemoved === 0`: `output.log('No deleted items to remove')`.
- else:
  - `✓ Compacted work unit <id>`
  - `  Removed items:`
  - if rules>0:     `    Rules: <n>`            (no chalk on Rules)
  - if examples>0:  `    Examples: <n>`         (chalk.dim)
  - if questions>0: `    Questions: <n>`        (chalk.dim)
  - if notes>0:     `    Architecture Notes: <n>` (chalk.dim)
- on error: `output.error(chalk.red('✗ Failed to compact work unit:'), error.message)` + exit 1.
- **The CLI action does NOT print the `--force` warning or the renumber summary** that the
  help-doc examples show. The action discards `result.warning`. (Framing A note: help doc
  examples diverge from actual CLI output; mirror the ACTUAL CLI action output, which is
  the contract enforced by byte-parity against `node dist/index.js`.)

## Rust port notes
- `rules`/`examples`/`questions`/`architectureNotes` arrays + `nextRuleId`/`nextExampleId`/
  `nextQuestionId`/`nextNoteId` counters all live in `WorkUnit.extra` (round-trip).
- `status` is a typed field on `WorkUnit` (`WorkUnitStatus`); use `.status.as_str()` for the
  error/warning message status string (lowercase canonical).
- `updatedAt` is typed (`WorkUnit.updated_at`); set via `iso8601_now()` (io::time).
- `meta.lastUpdated` is typed on `WorkUnitsData.meta` — update if present.
- Renumber must mutate the `id` field of each surviving item object in `extra`.
- Reference impls: `add_rule.rs` (extra-array mutation + write_json_atomic), `show_deleted.rs`
  (walking the four soft-delete arrays in canonical order), `remove_rule.rs` (soft-delete semantics).
- Two-front-doors: bridge `codelet/fspec/src/compact_work_unit.rs` marshals `{workUnitId, force?}`.
- Empty-arrays edge: counters reset to 0 when arrays absent/empty.

## Help fixture
`codelet/fspec/tests/fixtures/help/compact-work-unit.txt` — capture from
`node dist/index.js compact-work-unit --help` (non-TTY). Config has description, 1 arg,
1 option (--force), 2 examples, 7 notes, aiGuidance (5), 3 relatedCommands.
