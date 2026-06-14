# AST Research — `unlink-coverage` (RPC-311) — MUTATION command

## TS source of truth
- `src/commands/unlink-coverage.ts`
- `src/commands/unlink-coverage-help.ts` (rich help config — exists)
- coverage types: `src/utils/coverage-file.ts` → mirrored in `codelet/fspec-core/src/types/coverage.rs`

## Signature: `unlinkCoverage(featureName, options)`
options: `{ scenario, testFile?, implFile?, all=false, cwd }`

## Behaviour
1. Validate flag combos (ORDER matters):
   - `if (!all && !testFile)` → throw `Must specify either --all or --test-file\nUse --all to remove all mappings, or --test-file to remove specific test mapping`.
   - `if (implFile && !testFile)` → throw `--test-file is required when specifying --impl-file\nImplementation mappings are attached to test mappings`.
2. Resolve coverage file: `<cwd>/spec/features/<fileName>.coverage` where `fileName = featureName.endsWith('.feature') ? featureName : <name>.feature`.
3. Read + JSON.parse. On read/parse failure → throw `Coverage file not found: <fileName>.coverage\nSuggestion: Run fspec show-coverage to see available features`.
4. Find scenario by `coverage.scenarios.find(s => s.name === scenario)`. Not found → throw
   `Scenario not found: "<scenario>"\nAvailable scenarios:\n  - <name>\n  - ...`.
5. Mutation modes (priority order):
   - **--all**: `scenarioEntry.testMappings = []`. msg `✓ Removed all coverage mappings for scenario "<scenario>"`.
   - **--test-file + --impl-file**: find testMapping by `tm.file === testFile`; not found → throw
     `Test file not found in scenario mappings: <testFile>\nSuggestion: ...`.
     find implMapping index by `im.file === implFile`; -1 → throw `Implementation file not found in test mapping: <implFile>\nSuggestion: ...`.
     `splice(implIndex,1)`. msg `✓ Removed implementation mapping <implFile> from scenario "<scenario>"`.
   - **--test-file only**: find testIndex by `tm.file === testFile`; -1 → throw `Test file not found in scenario mappings: <testFile>\nSuggestion: ...`.
     `splice(testIndex,1)`. msg `✓ Removed test mapping <testFile> (and all its implementation mappings) from scenario "<scenario>"`.
6. `updateStats(coverage)` — recalculate:
   - coveredScenarios = scenarios with testMappings.length>0.
   - coveragePercent = totalScenarios>0 ? Math.round(covered/total*100) : 0.
   - testFiles = unique testMapping.file (Set insertion order).
   - implFiles = unique implMapping.file.
   - totalLinesCovered = totalTestLines + totalImplLines.
     - test lines: `lines.split('-')` → if 2 parts, end-start+1.
     - impl lines: `implMapping.lines.length` (lines is number[] here → array length!).
   - Note: this `updateStats` differs from show-coverage's calculate_stats (totalLinesCovered=0 there).
7. Atomic write via `fileManager.transaction(coverageFile, fileData => Object.assign(fileData, coverage))`.
8. Returns `{ success: true, message }`.

## `unlinkCoverageCommand(featureName, options)` (CLI action)
- success → `output.log(message)` + `process.exit(0)`.
- error → `output.error('Error:', msg)` + `process.exit(1)`.

## CLI registration
- `.command('unlink-coverage')`
- `.argument('<feature-name>', ...)` — REQUIRED positional.
- `.requiredOption('--scenario <name>', ...)` — REQUIRED.
- `.option('--test-file <path>', ...)`
- `.option('--impl-file <path>', ...)`
- `.option('--all', ...)` — boolean.

## Rust wiring intent
- Reuse `types::coverage::CoverageFile` / `CoverageScenario` / `TestMapping` / `ImplMapping` / `CoverageStats`.
- Read coverage sidecar with `std::fs::read_to_string` + `serde_json::from_str::<CoverageFile>`.
- Mutate in memory; recalculate stats — NOTE: must write a dedicated `update_stats` matching the
  unlink TS (totalLinesCovered computed; ImplLines::Array length OR String range split). The shared
  `calculate_stats` in types/coverage.rs sets totalLinesCovered=0 — DOES NOT match. Implement local update_stats.
- Write back atomically via `io::locked_file::write_json_atomic` (NO trailing newline — TS fileManager
  writes JSON.stringify(...,2) without trailing newline). preserve_order ensures field order; `extra`
  flatten preserves unknown fields.
- Errors via FspecCoreError (InvalidArgs for validation/not-found; Io for missing file).
- Help config exists → standard intercept arm. help-config common_errors type is `CommonError`.

## Edge cases / parity notes
- ImplMapping.lines is `number[]` per coverage-reader, but types/coverage.rs ImplLines is untagged
  Array|String. For totalLinesCovered: Array → len(); String "N-M" → ??? TS `implMapping.lines.length`
  on a string would give char length — but real impl mappings use number[]. Mirror: Array → vec.len();
  String → treat as 0 or char count? Keep Array→len, String→0 (document; real fixtures use arrays).
- testMapping.lines "45-62" → split('-'), 2 parts → 62-45+1=18. Single value or empty → 0.
- Atomic write back must NOT drop `stats.extra` / scenario `extra` / file `extra`.

## SHARED-FILE REQUESTS (for supervisor)
- canonical.rs: add `unlink-coverage` to PORTED_COMMANDS.
- dispatch.rs: route `unlink-coverage` → unlink_coverage::run; remove from run_stub.
- commands/mod.rs: (already declares module).
- help/configs/mod.rs: register unlink_coverage CONFIG.
- main.rs: Mode::UnlinkCoverage variant + forward! arm + intercept arm + `mod unlink_coverage;`.
- NOTE: unlink_coverage::run signature changes from `run(args_json)` → `run(args_json, project_root)`.
  dispatch.rs arm must pass project_root (supervisor wiring).
