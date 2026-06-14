# AST Research — validate (RPC-320)

## TS source of truth
- `src/commands/validate.ts` (266 LOC)
- `src/commands/validate-help.ts`

## TS control flow
### `validateCommand(file?, options?)`
1. `files = file ? [file] : await findAllFeatureFiles()` (glob `spec/features/**/*.feature`, relative).
2. If `files.length === 0` → `output.error('No feature files found in spec/features/')`, `process.exit(2)`.
3. `results = await Promise.all(files.map(f => validateFile(f, verbose)))`.
4. Display loop: valid → `✓ <file> is valid`; invalid → `✗ <file> has syntax errors:` then per error `  Line N: <message>` + optional `  Suggestion: <s>`.
5. Summary only when `results.length > 1`: blank line; all valid → `✓ All N feature files are valid`; else → `Validated N files: X valid, Y invalid`.
6. `if (invalidCount > 0) process.exit(1)`.
7. Outer catch → `output.error('Error:', message)`, `process.exit(2)`.

### `validateFile(filePath, verbose)` → ValidationResult `{file, valid, errors:[{line,message,suggestion?}]}`
1. `resolvedPath = resolve(process.cwd(), filePath)`; `readFile`.
2. verbose → `Parsing <file>...`.
3. Parse with `@cucumber/gherkin` (AstBuilder + GherkinClassicTokenMatcher + Parser).
4. On parse throw → `valid=false`, push `{line: parseError.location?.line || 0, message: parseError.message, suggestion: getSuggestion(message)}`, return.
5. `additionalErrors = checkForCommonIssues(content)` → if any, `valid=false`, push them.
6. verbose success logs feature name + scenario count.
7. catch ENOENT → `{line:0, message:'File not found: <path>'}`; else `{line:0, message}`.

### `checkForCommonIssues(content)` — content-string heuristics (NO parser)
- Tracks DocString boundaries (`"""`).
- Inside DocString: line includes `"""` and not `\\"""` → `Unescaped triple quotes (""") found inside DocString` + suggestion (escape or use ```).
- `>= 3` consecutive blank lines → `Excessive blank lines detected (N consecutive blank lines)` + suggestion 'Remove excess blank lines ...'. Skips ahead to avoid dupes.

### `getSuggestion(errorMessage)` — heuristics on lowercased message
- 'expected' & 'feature' → 'Add Feature keyword at the beginning of the file'
- 'unexpected'|'invalid' + ('while'|'whilst') → 'Use: Given, When, Then, And, or But'
- 'unexpected'|'invalid' + 'indent' → 'Check indentation - steps should be indented 2 spaces from Scenario'
- 'doc string'|'"""' → 'Add closing """'
- 'table' → 'Check data table formatting - each row must have same number of columns'
- else undefined

### `registerValidateCommand`
- `.command('validate').description('Validate Gherkin syntax in feature files').argument('[file]', '...').option('-v, --verbose', '...', false).action(validateCommand)`.

## Rust infrastructure to REUSE
- `io::feature_glob::glob_feature_files(project_root)` — recursive walk, sorted, returns relative forward-slash paths. NOTE: returns `Err(DirectoryNotFound)` when `spec/features/` is absent. TS findAllFeatureFiles uses tinyglobby which returns `[]` (no error) when the dir is missing → leads to exit 2 "No feature files found". MUST reconcile: catch DirectoryNotFound → treat as zero files → exit 2 path. (When a single file arg is given, glob is not used.)
- `io::gherkin::parse_feature_lenient(content) -> Result<Feature, ParseError>` — same parser the sibling Gherkin commands use.

## RPC-329 KNOWN DIVERGENCE (do NOT block)
- The embedded raw parser-error TEXT (`parseError.message`) differs: TS `@cucumber/gherkin` token vocabulary (`expected: #EOF, #StepLine, got '...'`) vs Rust `gherkin-0.16` (line:col + expected-token-set). A cucumber-compatible formatter in io/gherkin.rs is tracked under RPC-329, out of scope here.
- Tests assert STRUCTURAL facts only: file path, valid/invalid marker, exit code, presence of `Line N:`, Suggestion presence, the two content-heuristic messages (which DO match byte-for-byte because they're not parser-derived). Do NOT assert exact raw parser message.
- The error LINE NUMBER may also diverge (TS `parseError.location?.line` vs Rust crate position). Prefer asserting `Line ` substring presence over an exact number for parser errors; content-heuristic errors carry deterministic line numbers we control, so those CAN be asserted exactly.

## Open question (flagged to supervisor)
- Confirm test framing: structural + matching-substring assertions only for parser-error text (RPC-329). See work-unit question [0].

## New files this work unit produces (6 artifacts + features)
1. `codelet/fspec-core/src/commands/validate.rs` (rewrite stub → real impl; signature `run(args_json, project_root)`)
2. `codelet/fspec/src/validate.rs` (CLI bridge: `[file]` positional + `-v/--verbose`)
3. `codelet/fspec/src/main.rs` Mode::Validate variant + arm + intercept (SUPERVISOR-owned)
4. `codelet/fspec-core/src/help/configs/validate.rs`
5. `codelet/fspec/tests/fixtures/help/validate.txt`
6. `codelet/fspec/tests/cli_validate.rs`
+ core dispatcher test `codelet/fspec-core/tests/validate.rs`

## Shared-file change requests (SUPERVISOR)
- `canonical.rs`: add `validate` to PORTED_COMMANDS.
- `dispatch.rs`: arm `commands::validate::run(args_json)` → `run(args_json, project_root)`; move stub→ported.
- `help/configs/mod.rs`: register validate config.
- `main.rs`: `Mode::Validate { file: Option<String>, verbose: bool }`, forward! arm, intercept arm, `mod validate;`.

## Dispatcher output shape note
- TS validateCommand uses `process.exit(code)` directly; the dispatcher/CLI return a rendered String + exit code via the bridge. The core `run` must return a rendered String (the display block + summary). Exit code mapping (0/1/2) is the bridge's responsibility — but the dispatcher path only carries success/failure + data. DECISION: core `run` returns the rendered text; on the "no feature files" / unexpected-error path the core returns `Err(FspecCoreError)` so the bridge maps to exit 2 and writes the message to stderr. The 1-vs-0 distinction (some files invalid) needs to be carried in the result — propose core returns Ok(rendered) for the all-valid case and Err for the has-invalid case? NO — that conflates. PROPOSE: core returns a struct-ish JSON when format=json, and for text returns rendered + a sentinel. Simpler: bridge inspects whether rendered contains the invalid marker OR core returns Err with the rendered block as the message. NEEDS SUPERVISOR DECISION on exit-code transport (the list-* commands only had 0/1; validate has 0/1/2). Flag in report.
