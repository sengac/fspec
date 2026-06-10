# RPC-307 — AST Research: show-test-patterns

**TS Source:** `src/commands/show-test-patterns.ts` (~113 LOC)

## Exported Symbols (AstGrep)

```
src/commands/show-test-patterns.ts:30 export async function showTestPatterns(options)
src/commands/show-test-patterns.ts:73 export function registerShowTestPatternsCommand(program)
```

## Behaviour Map

1. `queryWorkUnits({tag: options.tag, cwd})` → returns `result.workUnits`.
2. `parseAllFeatures(options.cwd)` → list of parsed features (used to build featureByWorkUnit map; result not used in output).
3. `readAllCoverageFiles(options.cwd)` → glob `spec/features/*.feature.coverage` JSON files.
4. If `options.includeCoverage`:
   - `extractTestFiles(coverageFiles)` → unique list of testFile.filePath strings.
5. `patterns` is empty placeholder array — TODO in source.
6. Returns `{workUnits: [{tags: string[]}, ...], testFiles, patterns: [], format: 'json'|'table'}`.
7. CLI: if `--json` prints `JSON.stringify(result, null, 2)`; else prints green chalk message `✓ Analyzed testing patterns for N work units tagged with @TAG`.

## CLI Shape (commander)

```
fspec show-test-patterns
  --tag <tag>            REQUIRED. e.g. @high, @cli
  --include-coverage     Include test file paths
  --json                 JSON output
```

## Rust Port Plan

- `codelet/fspec-core/src/commands/show_test_patterns.rs::run(args_json, project_root)`
- Calls existing port `query_work_units` to get work units by tag.
- Needs new shared helper `codelet/fspec-core/src/io/coverage_glob.rs::read_all_coverage_files(project_root)` returning `Vec<CoverageFile>`.
- Needs `extract_test_files(&[CoverageFile])` helper (dedup'd test file path strings).
- Uses `gherkin_query::parse_all_features` (already shared) — but result currently unused in JSON output.
- Returns text or JSON via DispatchResult.
