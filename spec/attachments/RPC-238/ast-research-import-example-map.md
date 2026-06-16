# RPC-238 — import-example-map — AST / Port Research

## TS source
- `src/commands/import-example-map.ts` — `importExampleMap(workUnitId, file)`; inverse of
  `export-example-map` (RPC-228). Reads `spec/work-units.json`, validates the work unit exists and is in
  `specifying` state, appends rules/examples/questions/assumptions arrays, writes back.
- Canonical registry: `codelet/fspec-core/src/canonical.rs:391` (`ts_file: src/commands/import-example-map.ts`).

## Rust impl under test (already landed)
`codelet/fspec-core/src/commands/import_example_map.rs`:
- `struct ImportExampleMapArgs` (line 39) — `{ workUnitId: String, file: String }`, camelCase serde.
- `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` (line 45).
- `fn append_field(...)` (line 125) — appends one incoming array to the work-unit's existing array and
  returns the incoming count; missing/non-array category contributes 0 and leaves the existing array intact.
- `fn resolve_input(project_root: &Path, file: &str) -> PathBuf` (line 164) — resolves the import file path.

## Behaviour verified
- Reads `spec/work-units.json` via `ensure_work_units_file` (auto-create when missing, escalate malformed).
- Errors: `Work unit '<id>' does not exist`; `Can only import example mapping during discovery/specification
  phase. <id> is in '<status>' state.` when not in `specifying`.
- Reads `ExampleMapData { rules?, examples?, questions?, assumptions? }` (each optional array); APPENDS each
  present array; refreshes `updatedAt`; writes `work-units.json` (2-space).
- Returns `✓ Imported <total> items: <r> rules, <e> examples, <q> questions, <a> assumptions`.

## Two front doors (verified)
- Dispatcher arm `codelet/fspec-core/src/dispatch.rs:502` → `commands::import_example_map::run(args_json, project_root).await`.
- CLI bridge `codelet/fspec/src/import_example_map.rs` — two required positionals; on error prints
  `✗ Failed to import example map: <msg>` (exit 1).

## DRY / SOLID
- Reuses shared `ensure_work_units_file` + Value-based work-unit model; `append_field` is a single
  generic helper applied per category (no per-category duplication).
