# AST Research — `search-implementation` (RPC-296)

## TS source of truth
- `src/commands/search-implementation.ts`
- `src/commands/search-implementation-help.ts` (rich help config — exists)
- shared util: `src/utils/coverage-reader.ts` (`readAllCoverageFiles`, `extractImplementationFiles`)

## Behaviour (`searchImplementation(options)`)
1. `readAllCoverageFiles(cwd)`:
   - globs `spec/features/*.feature.coverage` (NON-recursive).
   - parse each JSON; on parse error → skip (`continue`).
   - returns `{ featureName (file w/o `.feature.coverage`), filePath: spec/features/<file>, scenarios: data.scenarios||[] }`.
2. `extractImplementationFiles(coverageFiles)`:
   - flattens scenarios → testMappings → `implMappings[]`.
   - emits `{ filePath: implMapping.file, scenarioName, featureName, lines: implMapping.lines }` per impl mapping.
3. For each impl file: `readFile(implFile.filePath, 'utf-8')`; if `content.includes(options.function)` → record
   `matchingFiles.set(filePath → Set<featureName>)`. Unreadable files skipped (`continue`).
4. Build result: for each matching filePath, re-read content, collect workUnitIds:
   - For each impl file with that filePath, `featureName.toUpperCase().replace(/-/g, '-')` (NO-OP regex; just uppercases).
     This is the "work unit ID" approximation. Dedup via Set.
   - `{ content, filePath, workUnits: [{workUnitId}] }`.
5. Returns `{ searchedFiles: implFiles.length, files }`.

## CLI registration
- `.command('search-implementation')`
- `.requiredOption('--function <name>', ...)` — REQUIRED
- `.option('--show-work-units', ...)` — boolean
- `.option('--json', ...)` — boolean
- action: success → `--json` → `JSON.stringify(result, null, 2)`; else green
  `✓ Found "<function>" in <N> file(s)`. Error → `output.error('✗ Search failed:', msg)` + exit 1.

## Rust wiring intent
- Reuse `io/coverage_glob.rs` — BUT it only extracts TEST refs (testMappings file), not impl mappings.
  Need impl mappings. Either: (a) extend coverage_glob with impl extraction (SHARED-FILE REQUEST), or
  (b) read coverage files inline via types/coverage.rs (CoverageFile → scenarios → testMappings → implMappings).
  PREFER (b): use `types::coverage::CoverageFile` and walk the dir inline (parity with show_test_patterns
  which inlined its own read). Document; submit optional shared-file request to add impl-extraction to coverage_glob.
- `implMapping.file` path is read relative to cwd/project_root via `std::fs::read_to_string(project_root.join(file))`.
  Note TS uses `readFile(implFile.filePath)` — filePath is relative to process.cwd(). Rust joins project_root.
- `content.includes(function)` → simple substring search (NOT regex despite help notes mentioning regex).
- workUnitId = featureName.to_uppercase() (the replace is a no-op).
- Dispatcher returns JSON envelope `{ searchedFiles, files }`; CLI bridge prints envelope or green summary.
- Help config exists → standard intercept arm.

## Edge cases
- Missing spec/features dir → coverage_glob returns [] → searchedFiles=0, empty files (no error).
- Impl files referenced but not on disk → skipped.
- Coverage parse errors → skipped.
- `files[].content` carries the FULL file content (large!). Mirror TS exactly for parity.
