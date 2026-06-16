# AST Research — delete-scenarios (RPC-220)

## TS source
- `src/commands/delete-scenarios-by-tag.ts` (350 LOC)
- `src/commands/delete-scenarios-by-tag-help.ts` — config `name: 'delete-scenarios'` BUT...

## ⚠️ HELP IS BARE-COMMANDER (special-case like delete-features)
`node dist/index.js delete-scenarios --help` does **NOT** use the rich `delete-scenarios-by-tag-help.ts` config. The registered Commander command (`registerDeleteScenariosCommand`) has no `.addHelpText` / custom action that invokes `formatCommandHelp`. The help-registry glob loads `*-help.ts` by FILENAME → `delete-scenarios-by-tag-help.ts` maps to command name `delete-scenarios-by-tag`, NOT `delete-scenarios`. So `delete-scenarios --help` falls through to bare Commander.js:

```
Usage: fspec delete-scenarios [options]

Bulk delete scenarios by tag across multiple files

Options:
  --tag <tag>  Filter by tag (can specify multiple times for AND logic)
  --dry-run    Preview deletions without making changes
  -h, --help   Display help for command
```

**→ Follow the delete-features bare-commander pattern**: hard-code a `DELETE_SCENARIOS_HELP: &str` const in main.rs, add an intercept arm that `print!`s it (no help-config module, NOT registered in configs/mod.rs). **SUPERVISOR ACTION REQUIRED** (shared main.rs).

Captured fixture: `codelet/fspec/tests/fixtures/help/delete-scenarios.txt` (8 lines).

## Core behaviour (`deleteScenariosByTag`)
1. glob `spec/features/**/*.feature` (relative). Empty → `{success:true, deletedCount:0, fileCount:0, message:'No feature files found'}`.
2. For each file: parse with @cucumber/gherkin (skip invalid syntax / no feature).
3. For each scenario child: collect scenario-level tags; match when ALL supplied tags ∈ scenarioTags (AND).
4. Compute lineStart = first scenario tag line (or scenario keyword line), lineEnd = next scenario/background start line (or EOF = lines.length).
5. totalScenarios==0 → `{success:true, deletedCount:0, fileCount:0, message:'No scenarios found matching tags'}`.
6. dryRun → `{success:true, deletedCount:total, fileCount:matchingFiles, message:'Would delete N scenario(s) from M file(s)', scenarios:[{file,name,tags,lineStart,lineEnd}]}`.
7. Real: per file, sort scenarios desc by lineStart, splice lines [lineStart-1 .. lineEnd-1). Join, collapse `\n{4,}`→`\n\n\n`. Re-parse; on failure return `{success:false, deletedCount:0, fileCount:0, error:'Validation failed after deleting scenarios from <file>: <msg>'}` (file NOT written). On success write file, filesModified++.
8. Coverage sidecar `<file>.coverage`: remove deleted scenario names, recalc stats {totalScenarios, coveredScenarios, coveragePercent=Math.round(...)}; missing/invalid coverage skipped silently.
9. Final: `{success:true, deletedCount:total, fileCount:filesModified, message:'Deleted N scenario(s) from M file(s). All modified files validated successfully.'}`.
10. Outer try/catch → `{success:false, deletedCount:0, fileCount:0, error: msg}`.

## CLI command (`deleteScenariosByTagCommand` + `registerDeleteScenariosCommand`)
- `--tag <tag>` repeatable (collector fn → string[])
- `--dry-run`
- Normalize tag: array | single | none→error 'At least one --tag is required' exit 1
- `!result.success` → `output.error('Error:', result.error)`, exit 1
- dryRun w/ scenarios:
  - `Dry run mode - no files modified`
  - cyan `\nWould delete N scenario(s) from M file(s):\n`
  - group by file → `\n<file>:` then `chalk.gray('  - <name> (<tags join ' '>)')`
- else → `✓ <message>`
- exit 0

### Verified CLI dry-run output (cat -A)
```
Dry run mode - no files modified$
$
Would delete 2 scenario(s) from 1 file(s):$
$
$
spec/features/a.feature:$
  - One (@spike)$
  - Two (@spike)$
```
Note: TWO blank lines between header and `<file>:` (one from the `\n...\n` header template, one from the `\n<file>:` per-file prefix).

## Rust port plan
- Core `delete_scenarios.rs::run(args_json, project_root)` → JSON envelope `Ok(String)` (compact); CLI bridge renders.
- Reuse `crate::io::feature_glob::glob_feature_files` and `crate::io::gherkin::parse_feature_lenient`.
- Gherkin AST line numbers: need scenario tag/keyword line + tags. gherkin-0.16 `Scenario` has `.position`/`span`? — TESTING phase will confirm available line-number API (Feature::parse gives `Span`/`LineCol`). Sibling: validate.rs uses gherkin crate. Tag line tracking may require manual line scan if crate lacks tag positions.
- Coverage sidecar update: read `<file>.coverage`, filter scenarios by name, recalc stats, write 2-space JSON.
- args: `{tags: Vec<String>, dryRun: bool}` (camelCase).
- CLI bridge marshals repeatable `--tag` → `{tags:[...], dryRun?}`; renders dry-run/real/error.

## SHARED-FILE CHANGE REQUESTS (supervisor)
1. canonical.rs: add `delete-scenarios` to PORTED_COMMANDS.
2. dispatch.rs: add run_ported arm, remove run_stub arm.
3. main.rs: add `Mode::DeleteScenarios { tag: Vec<String>, dry_run: bool }`, forward! arm, `mod delete_scenarios;`, **bare-commander intercept arm + DELETE_SCENARIOS_HELP const** (mirror delete-features).
4. commands/mod.rs: already registers `delete_scenarios` (stub present).
5. NO help/configs/mod.rs entry (bare-commander).
