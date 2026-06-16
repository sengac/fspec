# AST Research — compare-implementations (RPC-207)

## TS source
- `src/commands/compare-implementations.ts` (129 LOC)
- `src/commands/compare-implementations-help.ts` (rich CommandHelpConfig)

## Behaviour (TS)
1. `queryWorkUnits({tag, cwd})` reads `spec/work-units.json` (THROWS if missing → caught by command → `✗ Comparison failed:` + exit 1).
2. `result.workUnits || []` filtered to those whose `tags` array contains `--tag`.
3. `parseAllFeatures(cwd)` — parsed but only used to build a `featuresByWorkUnit` map that is never consumed (dead in TS). Rust can skip it; it has no effect on output.
4. `readAllCoverageFiles(cwd)` globs `spec/features/*.feature.coverage`.
5. If `--show-coverage`: `extractTestFiles` + `extractImplementationFiles` → dedup into `coverage[0] = {testFiles, implementationFiles}`. Otherwise `coverage` stays `[]`.
6. `namingConventionDifferences` always `[]` (TS TODO).
7. Returns `{ workUnits: [{tags}], comparison: {type:'side-by-side'}, namingConventionDifferences:[], coverage:[] }`.

## CLI command (`registerCompareImplementationsCommand`)
- `--tag <tag>` **requiredOption**
- `--show-coverage` flag
- `--json` flag
- Default (no --json): `chalk.green('✓ Compared N work units tagged with <tag>')`, exit 0
- `--json`: `JSON.stringify(result, null, 2)`
- Error: `output.error('✗ Comparison failed:', error.message)`, `process.exit(1)`

## Coverage reader shapes (`src/utils/coverage-reader.ts`)
- coverage file JSON: `{ scenarios: [{ name, testMappings: [{ file, lines, implMappings?: [{file, lines:number[]}] }] }] }`
- `extractTestFiles`: flatten testMappings.file
- `extractImplementationFiles`: flatten testMappings[].implMappings[].file
- both deduplicated by the command via `Set(...map(t => t.filePath))`

## Empty-workspace edge cases (verified via `node dist/index.js`)
- no spec dir + default text → `✗ Comparison failed: Failed to query work units: ENOENT...` exit 1
- no spec dir + `--json` → same error, exit 1 (queryWorkUnits throws before format branch)
- work-units.json present, 0 matches → `✓ Compared 0 work units tagged with @cli` exit 0

## Rust port plan
- Core: `compare_implementations.rs::run(args_json, project_root)` returns the JSON envelope as `Ok(String)` (compact OR pretty? — CLI bridge owns pretty-printing; core returns the **value**). Missing work-units.json → `Err(FspecCoreError::Io)` (parity with show_test_patterns precedent).
- Reuse `WorkUnitsData` type (tags live in `extra["tags"]`, see show_test_patterns).
- Coverage glob: inline a private reader (sibling precedent: show_test_patterns `read_test_refs`); read `spec/features/*.feature.coverage`.
- CLI bridge marshals `{tag, showCoverage?, json?}`; renders green summary / pretty JSON / `✗ Comparison failed:` stderr.
- **Help has a rich config** → normal `help/configs/compare_implementations.rs` module (NOT bare-commander).
- Two-front-doors parity test compares dispatcher JSON to CLI `--json` JSON.

## Note: COMMON PATTERNS "undefined" bug
The TS `-help.ts` uses `{title, commands}` object shape for commonPatterns, but help-formatter expects `{pattern, example, description}`. Result: the TS help renders literal `• undefined / Example: undefined / undefined` for the two pattern entries. **The captured fixture preserves this verbatim** — the Rust CONFIG must reproduce the same `undefined` output. Achieve byte-parity by encoding two `CommonPatternEntry::Structured{pattern:"undefined", example:"undefined", description:"undefined"}`.
