# RPC-197 — `audit-coverage` AST research (Rust port)

## TS source of truth
- `src/commands/audit-coverage.ts` (137 lines)
- `src/commands/audit-coverage-help.ts` (84 lines)

## Behaviour (verbatim from TS)
`auditCoverage({ featureName, cwd })`:
1. Resolves `<cwd>/spec/features/<featureName>.feature.coverage`.
2. **If the coverage file does NOT exist** → returns
   `{ output: "✗ Coverage file not found: <absolutePath>", exitCode: 1 }`.
   NOTE: the path printed is the FULL absolute path (`coverageFilePath`),
   not the bare basename.
3. Reads + `JSON.parse`s the file into `CoverageFile`.
4. Iterates `coverage.scenarios[].testMappings[]`:
   - pushes `testMapping.file` to `allFiles`; if `<cwd>/<file>` does not
     exist, pushes `{file, type:'test'}` to `missingFiles`.
   - for each `implMapping` in that test mapping: pushes `implMapping.file`
     to `allFiles`; if `<cwd>/<file>` missing, pushes `{file, type:'implementation'}`.
5. **If `missingFiles.length === 0`** → output is two lines:
   `✅ All files found (<n>/<n>)` + newline + `All mappings valid`,
   `exitCode: 0`.
6. **Otherwise** → header `✗ <m> missing file(s) out of <n> total files`
   followed by `\n\n`, then per missing file a 3-line block:
   - test:  `❌ Test file not found: <file>` + `\n` +
            `   Recommendation: Remove this mapping or restore the deleted file` + `\n\n`
   - impl:  `❌ Implementation file not found: <file>` + `\n` +
            `   Recommendation: Remove this mapping or restore the deleted file` + `\n\n`
   `exitCode: 1`.

`auditCoverageCommand(featureName)` calls `auditCoverage({featureName})`,
`output.log(result.output)`, then `process.exit(result.exitCode)`.

Commander registration (`registerAuditCoverageCommand`):
- `.command('audit-coverage')`
- `.argument('<feature-name>', '...')` — REQUIRED positional.
- NO options. (The help-doc advertises a `--fix` flag, but the TS CLI does
  NOT register it — see **Framing A** note below.)

## Framing A divergence (--fix)
`audit-coverage-help.ts` documents a `--fix` option and rich
"Auditing: ... ✓/✗ per scenario ..." output that the TS `auditCoverage`
implementation **does not produce**. The actual implementation emits the
compact `✅ All files found (n/n)` / `❌ ... not found` format with NO
`--fix` handling. Per command-port.md §10 "Help-text divergence policy":
**the IMPLEMENTATION output is what we port** (the help doc examples are
aspirational and not produced by the real CLI). We will mirror the actual
`auditCoverage` output byte-for-byte, and the `--help` fixture is captured
verbatim from `node dist/index.js audit-coverage --help` for the parity test.

## Shared Rust modules available (no new module needed)
- `crate::types::coverage::{CoverageFile, CoverageScenario, TestMapping, ImplMapping}`
  — ALREADY EXISTS (used by show-coverage RPC-300). `audit-coverage` only
  needs `scenarios[].testMappings[].{file, implMappings[].file}`. No stats.
- File existence check: `project_root.join(rel).exists()` (std::fs).
- Coverage-file path resolution: replicate show_coverage's
  `spec/features/<name>.feature.coverage` (strip trailing `.feature` if present —
  TS does NOT strip, but show-coverage does; **audit-coverage TS does NOT strip**,
  so we mirror audit-coverage exactly: `<name>.feature.coverage` with raw name).

## Exit-code transport
Core `run` returns a JSON envelope `{ output, exitCode }` (same pattern as
`validate.rs`). The CLI bridge parses it, prints `output` to stdout, returns
`exitCode`. The not-found case (exit 1) is NOT an `Err` — it is a normal
`{output, exitCode:1}` payload because the TS returns it via the result object,
not a throw. Malformed JSON → `JSON.parse` throws in TS → uncaught → Node exits
non-zero with a stack trace; in Rust we map a parse error to
`FspecCoreError` (bridge → exit 1, stderr) for graceful parity.

## Output declaration order
Use `serde_json::json!({ "output": ..., "exitCode": ... })` — only two scalar
keys, order is trivially preserved (and the CLI consumes by key name, not order).
