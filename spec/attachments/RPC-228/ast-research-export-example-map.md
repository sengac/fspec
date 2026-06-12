# AST Research — export-example-map (RPC-228)

## TS source of truth
- `src/commands/export-example-map.ts` (91 LOC)
- `src/commands/export-example-map-help.ts` (help config)

## Behaviour (TS)
`exportExampleMap({ workUnitId, file, cwd? })`:
1. `cwd = options.cwd || process.cwd()`.
2. `data = await ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json` if missing; escalates malformed JSON.
3. If `!data.workUnits[workUnitId]` → `throw new Error("Work unit '<id>' does not exist")`.
4. Build `exportData`:
   ```json
   {
     "workUnitId": "<id>",
     "title": "<workUnit.title>",
     "rules": workUnit.rules || [],
     "examples": workUnit.examples || [],
     "questions": workUnit.questions || [],
     "assumptions": workUnit.assumptions || []
   }
   ```
   Field order is FIXED: workUnitId, title, rules, examples, questions, assumptions.
5. `mkdir(dirname(outputFile), { recursive: true })`.
6. `writeFile(outputFile, JSON.stringify(exportData, null, 2))` — 2-space indent, NO trailing newline.
7. Returns `{ success, outputFile, rulesCount, examplesCount, questionsCount, assumptionsCount }`.

## CLI registration
- `program.command('export-example-map')`
- `.argument('<workUnitId>', 'Work unit ID')` — required positional
- `.argument('<file>', 'Output JSON file path')` — required positional
- Action: on success `output.log('✓ Exported to ${result.outputFile}')`.
  - NOTE: success message is **`✓ Exported to <file>`** (NOT "Exported Example Mapping to..."; that string only appears in the help EXAMPLE output line).
- On error: `output.error(chalk.red('✗ Failed to export example map:'), error.message)` + `process.exit(1)`.

## Verified actual output (`node dist/index.js`)
- Success stdout (piped, non-TTY): `✓ Exported to emap.json\n` (single trailing newline, chalk identity)
- Nonexistent WU: exit=1, stderr `✗ Failed to export example map: Work unit 'NOPE-999' does not exist`
- Missing `file` arg: exit=1, stderr `error: missing required argument 'file'` (Commander.js)
- Empty WU (no example-map fields): emits `"rules": []`, `"examples": []`, `"questions": []`, `"assumptions": []`.
- Item objects round-trip verbatim from disk (id/text/deleted/selected/createdAt etc.).

## Item shapes (src/types/index.ts)
- `ItemWithId`: `{ id: number; text: string; deleted: boolean; createdAt: string; deletedAt?: string }`
- `RuleItem = ItemWithId`, `ExampleItem = ItemWithId`, `ArchitectureNoteItem = ItemWithId`
- `QuestionItem extends ItemWithId { selected: boolean; answered?: boolean; answer?: string }`
- `assumptions: string[]`

## Rust port plan
- Read these fields out of `WorkUnit.extra` as `serde_json::Value` (NOT typed): `rules`, `examples`, `questions`, `assumptions`. Default to empty array `[]` if missing/non-array → preserve verbatim Values to keep round-trip parity.
- `ensure_work_units_file(project_root)` (auto-create + escalate parse error) — matches `ensureWorkUnitsFile`.
- Build output via a `#[derive(Serialize)]` struct (workUnitId, title, rules, examples, questions, assumptions) to guarantee declaration field order; serialize with `serde_json::to_string_pretty` (2-space) — must NOT alphabetize.
- Write to `<file>`, creating parent dirs (`std::fs::create_dir_all(dirname)`).
- Core `run` returns the success message string `✓ Exported to <file>` (CLI bridge prints it). Mirror query-work-units bridge for stdout handling.
- Error: missing WU → `FspecCoreError::InvalidArgs` carrying `Work unit '<id>' does not exist`; bridge prints `✗ Failed to export example map: <msg>` to stderr, exit 1.

## Reference impls consulted
- `codelet/fspec-core/src/commands/query_dependency_stats.rs` (reads WorkUnit deps + extra arrays)
- `codelet/fspec-core/src/io/ensure.rs` (`ensure_work_units_file`)
- `codelet/fspec/src/query_work_units.rs` (CLI bridge pattern)
- `codelet/fspec-core/src/types/work_unit.rs` (WorkUnit.extra)
- `codelet/fspec-core/src/help/configs/query_work_units.rs` (help config shape)
