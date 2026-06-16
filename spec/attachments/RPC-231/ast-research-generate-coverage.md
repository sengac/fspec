# RPC-231 — generate-coverage — AST / Port Research (Worker 3)

## TS source
- `src/commands/generate-coverage.ts` — `generateCoverage(options)` + `generateCoverageCommand` + `registerGenerateCoverageCommand`.
- `src/commands/generate-coverage-help.ts` — rich help config.
- Delegates the real work to `src/utils/coverage-file.ts` → `createCoverageFile(featureFilePath)`.

## Behaviour (non-dry-run)
For each `*.feature` file under `spec/features/`:
- Call `createCoverageFile(featureFilePath)` which:
  - If `<feature>.coverage` does NOT exist → parse feature, extract scenario names, write `{ scenarios:[{name,testMappings:[]}...], stats:{totalScenarios,coveredScenarios:0,coveragePercent:0,testFiles:[],implFiles:[],totalLinesCovered:0} }` → status `created`.
  - If exists + valid JSON → `updateCoverageFile`: add new scenarios (empty testMappings), DROP stale scenarios not in feature file; if no change → `skipped`; else recompute stats (totalScenarios, coveredScenarios, coveragePercent only — preserves other stats via spread) and write → `updated`.
  - If exists + invalid JSON → recreate (same as created body) → `recreated`.
- Tally counts: created / updated / skipped / recreated.
- Output line: `✓ Created N, Updated N, Skipped N, Recreated N (invalid JSON)` joined by `, ` for nonzero parts; if all zero → `No coverage files needed`.
- ALWAYS append the long `<system-reminder>` block (verbatim, see TS lines 155-189).
- Missing `spec/features/` dir → throw `Failed to read features directory: <msg>` (exit 1).

## Behaviour (dry-run)
- Does NOT call createCoverageFile. For each feature file checks the `.coverage`:
  - missing → `created++`, push name
  - exists + valid → `skipped++`
  - exists + invalid → `recreated++`, push name
- Output: `Would create N coverage files (DRY RUN)`, then `Files that would be created:` list, then `Would skip N existing files` / `Would recreate N invalid files` lines. NOTE: dry-run never counts "updated" (no scenario-diff in dry-run).

## Rust port plan
- Rewrite stub `codelet/fspec-core/src/commands/generate_coverage.rs`: `pub async fn run(args_json: &str, project_root: &Path)`.
  - SHARED-FILE REQUEST: dispatch arm signature must change `run(args_json)` → `run(args_json, project_root)` (supervisor edit to dispatch.rs).
- Reuse `types::coverage` (CoverageFile/CoverageScenario/CoverageStats) — REUSE, no extension needed.
- Need scenario-name extraction: use `io::gherkin::parse_feature_lenient(&content)` then iterate `feature.scenarios` (+ `feature.rules[*].scenarios`) reading `.name`. Mirror TS which only reads top-level `child.scenario`; but lenient/Rust commands generally include rule-nested. Will mirror TS: top-level scenarios only to keep parity (TS iterates `feature.children` `child.scenario`). DECISION: top-level scenarios only.
- Write sidecars via `io::locked_file::write_json_atomic` (2-space JSON, no trailing newline) — same as unlink-coverage.
- Args: `{ dryRun?: bool }`.
- Return an envelope `{ created, updated, skipped, recreated, dryRun, files? }` and let CLI bridge render the human output + system-reminder. ALTERNATIVELY render in core. DECISION: render the full stdout string (counts line + system-reminder) in CORE and return it (parity with show-coverage which renders in core); CLI bridge prints verbatim. This keeps the system-reminder text in one place.

## Stats helper note
`calculate_stats` in types::coverage sets `totalLinesCovered=0` and computes covered/percent — matches the empty-create path. For `updated` we only need totalScenarios/coveredScenarios/coveragePercent recompute preserving existing stats; can build inline like delete_scenario's update_coverage (Value-based) OR typed. DECISION: typed CoverageFile round-trip; recompute via calculate_stats is acceptable since new scenarios have empty mappings (covered count unchanged) — but to preserve byte parity with TS spread (which keeps testFiles/implFiles/totalLinesCovered untouched on update) we will mutate only the three fields on the existing stats Value. Use serde_json::Value path like delete_scenario::update_coverage to avoid dropping fields.

## No new shared helpers required beyond dispatch signature change.
