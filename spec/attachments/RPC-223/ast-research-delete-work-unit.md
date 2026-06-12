# AST Research — `delete-work-unit` (RPC-223)

TS source of truth: `src/commands/delete-work-unit.ts` (+ `delete-work-unit-help.ts`).
Rust target: `codelet/fspec-core/src/commands/delete_work_unit.rs` (currently a `NotYetPorted` stub).

## TS behaviour (src/commands/delete-work-unit.ts)

Options (`DeleteWorkUnitOptions`):
- `workUnitId: string` (required positional `<workUnitId>`)
- `force?: boolean`            — declared (`--force`) but **never read** by impl (parity-ignore like delete-epic `--force`)
- `skipConfirmation?: boolean` — `--skip-confirmation` declared but **never read** by impl (confirmation prompt is not actually implemented in the function)
- `cascadeDependencies?: boolean` — `--cascade-dependencies`, the ONLY behaviourally-significant flag
- `cwd?: string`               — defaults to `process.cwd()`

Result (`DeleteWorkUnitResult`): `{ success: boolean; warnings?: string[] }`
(`warnings` omitted entirely when empty — TS `...(warnings.length > 0 && { warnings })`).

### Algorithm (in order)
1. `ensureWorkUnitsFile(cwd)` — load-or-init (auto-creates `spec/work-units.json`). → Rust `ensure_work_units_file`.
2. Path = `join(cwd, 'spec/work-units.json')`.
3. **Existence check**: if `!workUnitsData.workUnits[id]` → throw `Work unit '<id>' does not exist`.
4. **Children check**: if `workUnit.children?.length > 0` → throw
   `Cannot delete work unit with children: <c1, c2>. Delete children first or remove parent relationship.`
5. **Dependency check**: `hasDependencies = blocks|blockedBy|dependsOn|relatesTo any non-empty`.
   If `hasDependencies && !cascadeDependencies` → throw
   `Work unit '<id>' has dependencies. Use --cascade-dependencies flag to remove dependencies and delete.`
6. **Warning**: if `workUnit.blocks?.length > 0` → push warning
   `This work unit blocks <n> work unit(s): <id1, id2>`.
   (NOTE: warning is computed even WITHOUT cascade — but step 5 would have thrown first
   unless `cascadeDependencies` is set. So warning only surfaces when cascading.)
7. **Cascade cleanup** (only if `cascadeDependencies`): bidirectional dereference —
   - For each `t` in `workUnit.blocks`: remove `id` from `workUnits[t].blockedBy`; delete `blockedBy` if now empty.
   - For each `t` in `workUnit.blockedBy`: remove `id` from `workUnits[t].blocks`; delete if empty.
   - For each `t` in `workUnit.relatesTo`: remove `id` from `workUnits[t].relatesTo`; delete if empty.
   - (NOTE: `dependsOn` is checked for hasDependencies but NOT cleaned up bidirectionally — TS has no dependsOn cascade block. Preserve this asymmetry exactly.)
8. **Parent cleanup**: if `workUnit.parent` and that parent exists → filter `id` out of `parent.children`.
9. **States index cleanup**: for every state array in `workUnitsData.states`, filter out `id`.
10. **Delete**: `delete workUnitsData.workUnits[id]`.
11. **Atomic write**: `fileManager.transaction(file, data => Object.assign(data, workUnitsData))`.
    → Rust: single `write_json_atomic(&path, &data)`.
12. Return `{ success: true, ...(warnings) }`.

### CLI registration (action)
- Success: `output.log(chalk.green('✓ Work unit <id> deleted successfully'))`
- Then for each warning: `output.log('⚠ <warning>')`
- On error: `output.error(chalk.red('✗ Failed to delete work unit:'), error.message)` + `process.exit(1)`.

## Rust port notes
- `children`, `blocks`, `blockedBy`, `dependsOn`, `relatesTo`, `parent` all live in
  `WorkUnit.extra` (NOT typed). Read as `extra.get(key).and_then(Value::as_array)`.
- `parent` is a string in `extra`.
- States: typed `WorkUnitStates` (backlog…blocked). Filter each Vec<String>.
- Reference impl shape: `delete_epic.rs` (delete-on-store + dereference side-effects + atomic write).
- Two-front-doors: bridge `codelet/fspec/src/delete_work_unit.rs` marshals
  `{workUnitId, cascadeDependencies?}` JSON only (force/skipConfirmation accepted but not forwarded — parity-ignore).
- Error prefix on CLI stderr: `✗ Failed to delete work unit: <msg>`.
- Success render returns `✓ Work unit <id> deleted successfully\n` + optional `⚠ <warning>\n` lines.

## Help fixture
`codelet/fspec/tests/fixtures/help/delete-work-unit.txt` — capture from
`node dist/index.js delete-work-unit --help` (non-TTY). Config has whenToUse,
prerequisites, 3 options, 4 examples, 3 commonErrors, typicalWorkflow, 2 commonPatterns,
6 relatedCommands, 8 notes.
