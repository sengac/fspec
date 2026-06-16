# RPC-201 — `check` AST research (Rust port)

## TS source of truth
- `src/commands/check.ts` (235 lines)
- `src/commands/check-help.ts` (32 lines)

## Behaviour (verbatim from TS `check({ verbose, cwd })`)
1. Glob `spec/features/**/*.feature` (relative, cwd). If ZERO files →
   return `{ success: true, message: 'No feature files found', fileCount: 0 }`.
2. `fileCount = files.length`.
3. **Gherkin syntax check** (`gherkinStatus`): parse each file with
   `@cucumber/gherkin`. Any parse failure → `gherkinStatus = 'FAIL'` and push
   `Gherkin syntax error in <file>: <message>` to errors.
4. **Tag validation** (`tagStatus`): call `validateTags({ cwd })`. If
   `invalidCount > 0` → `tagStatus = 'FAIL'` and push each per-file per-tag
   `error.message`. Thrown error → `tagStatus = 'FAIL'` + `Tag validation error: <msg>`.
5. **Formatting check** (`formatStatus`): for each file, parse → `formatGherkinDocument`
   → compare with original content. If different → `formatStatus = 'FAIL'` and push
   `Formatting check failed: <file> needs formatting`. Files that fail to parse are
   silently skipped here (already caught in step 3). An outer throw → `formatStatus = 'SKIP'`.
6. `success = gherkinStatus !== 'FAIL' && tagStatus !== 'FAIL' && formatStatus !== 'FAIL'`.
7. Returns `{ success, gherkinStatus, tagStatus, formatStatus, fileCount, errors?, message?, details? }`.
   - `message = 'All checks passed'` only when `success`.
   - `errors` omitted when empty.
   - When `verbose`, `details = { files, gherkinChecked, tagsChecked, formattingChecked }`.

## `checkCommand({ verbose })` rendering (the CLI shell)
```
\nRunning validation checks...\n
[Checked <n> feature file(s)\n   — only when fileCount > 0]
Gherkin syntax: <PASS|FAIL|SKIP>
Tag validation: <PASS|FAIL|SKIP>
Formatting: <PASS|FAIL|SKIP>
[\nErrors:\n  - <err>  — for each error, only when errors present]
<blank line>
✓ <message>   (when success)
✗ Some checks failed   (when not success)
```
Then `process.exit(success ? 0 : 1)`.

Commander registration (`registerCheckCommand`):
- `.command('check')`
- `.option('-v, --verbose', '...', false)`
- NO positional args.

## Shared-infrastructure dependencies (SUPERVISOR ATTENTION)
`check` is a COMPOSITE of three sub-checks. Two of the three already have Rust ports:
1. **Gherkin syntax** → reuse `crate::io::gherkin::parse_feature_lenient`
   (used by validate.rs RPC-320). ✅ EXISTS.
2. **Tag validation** → reuse `crate::commands::validate_tags::run` (RPC-324)
   which returns the `{results, validCount, invalidCount}` envelope. We can call
   it internally and extract `invalidCount` + per-error messages. ✅ EXISTS.
   NOTE: validate_tags::run is `async fn run(args_json, project_root)` — calling it
   internally from board/check is fine (same crate). Need to await it (resolves on
   first poll under the sync-dispatch model).
3. **Formatting check** → requires a Gherkin AST→text FORMATTER
   (`formatGherkinDocument` / `src/utils/gherkin-formatter.ts`, ≈380 LOC).
   **THIS DOES NOT EXIST IN RUST YET.** The `format` command (RPC-230) is still a
   NotYetPorted stub, and no `format_gherkin`/`GherkinFormatter` exists under
   codelet/fspec-core/src/.

### DECISION REQUEST for SUPERVISOR (blocking for full check parity)
The formatting sub-check needs a shared Gherkin formatter module. Options:
  (A) Supervisor (or a dedicated work unit) ports `gherkin-formatter.ts` to a new
      shared module `codelet/fspec-core/src/io/gherkin_format.rs` FIRST, then check
      (and format RPC-230) both consume it. This is the clean path but adds a
      cross-worker dependency.
  (B) Port `check` now with gherkinStatus + tagStatus fully implemented, and treat
      formatStatus as **SKIP** until the formatter module lands (the TS code itself
      sets formatStatus='SKIP' on an outer throw, so SKIP is a legitimate state and
      does NOT fail `success`). Document as a tracked partial-port divergence; wire
      the real formatter in a follow-up once RPC-230's formatter exists.

I RECOMMEND (B) for this batch: it lets check land its two real sub-checks with full
parity now, keeps `success` semantics correct (SKIP never fails), and avoids blocking
on a 380-LOC formatter port owned elsewhere. The feature scenarios will assert
gherkin + tag behaviour precisely and assert formatStatus is reported (PASS only once
the formatter lands; SKIP meanwhile). **Awaiting supervisor confirmation of A vs B.**

## Exit-code transport
Core `check::run` returns the full JSON result object (success/gherkinStatus/
tagStatus/formatStatus/fileCount/errors/message/details). The CLI bridge renders the
`checkCommand` display block from those fields and exits `success ? 0 : 1`. (validate.rs
+ validate_tags.rs precedent: core returns structured JSON, bridge renders + sets exit.)

## Shared modules
- `crate::io::feature_glob::glob_feature_files` — EXISTS (DirectoryNotFound → empty).
- `crate::io::gherkin::parse_feature_lenient` — EXISTS.
- `crate::commands::validate_tags::run` — EXISTS (call internally).
- Gherkin formatter — MISSING (see decision above).
