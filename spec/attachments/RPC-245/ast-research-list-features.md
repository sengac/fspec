# AST Research — RPC-245 `list-features` Rust Port

Generated: 2026-06-04
Worker: parallel orchestration agent (Phase A)

## TypeScript source under port

`src/commands/list-features.ts` (158 LOC)

## Top-level entities

| Pattern                                                  | AST hit                        |
|----------------------------------------------------------|--------------------------------|
| `interface FeatureInfo { file, name, scenarioCount, tags }` | `list-features.ts:10-15` |
| `interface ListFeaturesOptions { cwd?, tag? }`           | `list-features.ts:17-20`       |
| `interface ListFeaturesResult { features: FeatureInfo[] }` | `list-features.ts:22-24`     |
| `export async function listFeatures(options)`            | `list-features.ts:26-102`      |
| `export async function listFeaturesCommand(options)`     | `list-features.ts:104-150`     |
| `export function registerListFeaturesCommand(program)`   | `list-features.ts:152-157`     |

## Control flow of `listFeatures`

1. `cwd = options.cwd || process.cwd()` — `list-features.ts:29`
2. `featuresDir = join(cwd, 'spec', 'features')` — `list-features.ts:30`
3. `access(featuresDir)`:
   - ENOENT → `throw new Error('Directory not found: spec/features/')`
   - EACCES → `throw new Error('Permission denied: Cannot access spec/features/\nSuggestion: Check directory permissions')`
   - other → `throw new Error('Failed to access directory: <msg>')`
4. `glob(['spec/features/**/*.feature'], { cwd, absolute: false })` — `list-features.ts:48-51`
5. If `files.length === 0` → `return { features: [] }` — `list-features.ts:53-55`
6. For each file:
   - `readFile(join(cwd, file), 'utf-8')`
   - Parse via `@cucumber/gherkin` `Parser` with classic token matcher (`list-features.ts:65-70`)
   - If parsed `gherkinDocument.feature` exists:
     - `tags = feature.tags.map(t => t.name)` — tag names INCLUDE the leading `@`.
     - **Filter by `options.tag`**: `if (options.tag && !tags.includes(options.tag)) continue;`
     - `scenarioCount = feature.children.filter(c => c.scenario !== undefined).length`
     - Push `{ file, name, scenarioCount, tags }`
   - On parse error: `output.warn('Warning: Could not parse <file>')` — file is skipped, no throw.
7. Sort: `features.sort((a, b) => a.file.localeCompare(b.file))` — alphabetical by file path.
8. Return `{ features }`.

## CLI surface (Commander.js)

```ts
program
  .command('list-features')
  .description('List all feature files')
  .option('--tag <tag>', 'Filter by tag (e.g., --tag=@critical)')
  .action(listFeaturesCommand);
```

**One option registered:** `--tag <tag>`. No `--format`, no `--cwd`.

## CLI text rendering (`list-features.ts:104-150`)

- Empty result: `output.log('No feature files found in spec/features/')` → `process.exit(0)`
- Populated rendering for each feature:
  - `  <chalk.blue(file)> - <name> <chalk.gray("(<N> scenarios)")><chalk.gray(tagsStr)>`
  - `tagsStr` = `' [' + tags.join(' ') + ']'` when `tags.length > 0`, else empty.
- Summary line (after blank line):
  - With filter: `chalk.green('Found <N> feature files matching <tag>')`
  - Without filter: `'Found <N> feature files'`
- Error path:
  - If error message includes `'Directory not found'` → `output.error(msg)` + suggestion `chalk.gray("  Suggestion: Run 'fspec create-feature' to create your first feature")` → `process.exit(2)`
  - Otherwise → `output.error('Error:', msg)` → `process.exit(1)`

## Categorisation

**Category C (Read/Query)** with **gherkin parsing**.

## Rust Port Plan

- New shared helper module: `codelet/fspec-core/src/io/feature_glob.rs` for `glob_feature_files(cwd) -> Result<Vec<String>>`
- Add `gherkin` dependency to `codelet/fspec-core/Cargo.toml` (`gherkin = "0.16"` with `parser` feature).
- New module `codelet/fspec-core/src/commands/list_features.rs` with:
  - `ListFeaturesArgs { tag: Option<String>, format: Option<String> }` — `format` exposes JSON for the dispatcher path.
  - `FeatureInfo { file, name, scenario_count, tags }` (camelCase serde rename).
  - `run(args_json, project_root) -> Result<String, FspecCoreError>`.
- CLI bridge `codelet/fspec/src/list_features.rs` with `CliArgs { tag: Option<String> }`, delegating to `fspec_core::commands::list_features::run`.

## Shared-file changes required for Phase C

1. **`codelet/fspec-core/Cargo.toml`** — add `gherkin = { version = "0.16", default-features = false, features = ["parser"] }`.
2. **`codelet/fspec-core/src/io/mod.rs`** — add `pub mod feature_glob;` (new file).
3. **`codelet/fspec-core/src/canonical.rs`** — add `"list-features"` to `PORTED_COMMANDS`.
4. **`codelet/fspec-core/src/dispatch.rs`** — add ported arm for `"list-features"` and remove the stub-path entry.
5. **`codelet/fspec/src/main.rs`** — add `mod list_features;`, add `Mode::ListFeatures { tag: Option<String> }` variant, wire arm.
6. **`codelet/fspec/tests/cargo_shape.rs`** — extend lock-list (8 → 9 entries) to include `list_features.rs`.

## Gotchas / Parity Concerns

- Tag filter MUST match the leading `@` character. TypeScript stores tag names WITH `@` (e.g. `@critical`). Filter comparison is exact equality.
- `feature.children.filter(c => c.scenario !== undefined)` — in Rust `gherkin` crate, this maps to `feature.scenarios.len()` (Background is on `feature.background`, Rule is on `feature.rules`). Scenarios inside Rules are NOT counted by TS (it filters top-level children only).
- `localeCompare` ≈ Rust's `cmp` on UTF-8 strings is usually adequate for ASCII paths.
- Parse errors are SWALLOWED (not escalated) — files that fail to parse are silently skipped.
- `Directory not found` error MUST escalate (the dispatcher returns success=false). CLI surface returns exit code 2 (not 1) for this specific error.
- Empty result (zero matching feature files) prints sentinel `'No feature files found in spec/features/'` to stdout and exits 0.
