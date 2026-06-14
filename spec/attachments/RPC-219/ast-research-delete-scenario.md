# AST Research — delete-scenario (RPC-219)

## TS source: `src/commands/delete-scenario.ts`

Exported API:
- `deleteScenario(options: { feature, scenario, cwd? }): Promise<{ success, message?, error? }>`
- `deleteScenarioCommand(feature, scenario)` — Commander action (exit 0/1)
- `registerDeleteScenarioCommand(program)` — `delete-scenario <feature> <scenario>`

## Behaviour walk-through

1. **Path resolution** (lines 27-34):
   - if `feature` ends with `.feature` → `join(cwd, feature)`
   - else if starts with `spec/features/` → `join(cwd, feature)`
   - else → `join(cwd, 'spec/features', feature + '.feature')`
2. **Read** (lines 37-48): ENOENT → `success=false, error="Feature file not found: <absPath>"`. Other IO errors throw.
3. **Parse Gherkin** (lines 51-71): parse error → `error="Invalid Gherkin syntax: <msg>"`; no `feature` → `error="Feature file does not contain a valid Feature"`.
4. **Find scenario** (74-83): match `child.scenario.name === scenario`. Not found → `error="Scenario '<scenario>' not found in feature file"`.
5. **Compute span** (86-95): start = scenario location line; end = last step location line (or scenario line if no steps).
6. **Extend end past trailing blanks** (101-121): from `scenarioEndLine` forward, stop at next `Scenario:`/`Scenario Outline:`/`Background:`/`Feature:`/`Examples:`; include trailing blank lines as part of removal; stop at first non-blank non-header content after end line.
7. **Slice removal** (124-130): `startIndex = scenarioStartLine - 1`, `endIndex = actualEndLine` (inclusive). `newLines = lines[0..startIndex] + lines[endIndex+1..]`.
8. **Collapse blank runs** (133-145): allow at most 2 consecutive empty lines.
9. **Re-validate** (150-160): re-parse joined content; on parse error → `error="Deletion would result in invalid Gherkin: <msg>"`.
10. **Write** (163): `writeFile(featurePath, newContent)`.
11. **Coverage update** (166-212):
    - read `<featurePath>.coverage`; if missing/invalid → still succeed, message = `Successfully deleted scenario '<scenario>' from <fileName>`.
    - else: filter `coverage.scenarios` removing `s.name === scenario`; recompute stats `totalScenarios`, `coveredScenarios` (testMappings.length>0), `coveragePercent` (Math.round(covered/total*100) or 0); write pretty JSON; message = `Successfully deleted scenario '<scenario>' from <fileName>\n  Updated coverage file`.

## Rust mapping

- Reuse `crate::io::gherkin::parse_feature_lenient`.
- gherkin-0.16 `Scenario.position.line` (1-based) and `Step.position.line`.
- Coverage: reuse `crate::types::coverage::{CoverageFile, CoverageScenario, calculate_stats}`.
  - NOTE: TS only recomputes the 3 stat fields, preserving other stat fields via spread `...coverage.stats`. Use `extra` flatten to preserve. `calculate_stats` recomputes ALL fields incl testFiles/implFiles — may diverge. Prefer manual recompute of the 3 fields + preserve `extra` to match TS.
- `fileName` = last path segment (`split('/').pop()`).
- Error envelope JSON: `{ success: false, error }`; success: `{ success: true, message }`.

## Shared-file needs
- None expected. `coverage` types + `gherkin` io already public. dispatch arm currently 1-arg `commands::delete_scenario::run(args_json)` — supervisor must change to 2-arg `(args_json, project_root)`.
