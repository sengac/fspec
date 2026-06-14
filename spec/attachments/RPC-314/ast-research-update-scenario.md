# AST Research — update-scenario (RPC-314)

## TS source: src/commands/update-scenario.ts
Line-based rename of a scenario in a `.feature` file + coverage-entry rename.

### Behaviours (rule candidates)
1. **Feature-path resolution** (3-way): if ends with `.feature` → `join(cwd, feature)`;
   else if starts with `spec/features/` → `join(cwd, feature)`; else
   `join(cwd, 'spec/features', feature + '.feature')`.
2. **File-not-found** (ENOENT) → `{success:false, error:"Feature file not found: <abs>"}`.
   Other read errors rethrow.
3. **Invalid Gherkin** → `{success:false, error:"Invalid Gherkin syntax: <msg>"}`.
4. **No Feature element** → `{success:false, error:"Feature file does not contain a valid Feature"}`.
5. **Scenario (old-name) not found** → `{success:false, error:"Scenario '<old>' not found in feature file"}`.
   Matches top-level scenario children by exact name.
6. **Duplicate new-name** already present → `{success:false, error:"Scenario '<new>' already exists in this feature"}`.
7. **Locate header line** via `scenario.location.line` (1-based), convert to 0-indexed.
   Regex `/^(\s*)(Scenario|Scenario Outline):\s*(.+)$/` to capture indent + keyword.
   If regex fails → `{success:false, error:"Could not parse scenario header line"}`.
8. **Replace** header line with `${indent}${keyword}: ${newName}` (preserve indent + keyword).
9. **Re-validate** new content as Gherkin; if invalid → `{success:false,
   error:"Renaming would result in invalid Gherkin: <msg>"}`.
10. **Write** updated content (split('\n')/join('\n') line-based — preserves endings).
11. **Coverage rename**: read `<featurePath>.coverage`, JSON.parse, find scenario entry by
    old name, set `.name = newName`, write back pretty (2-space). Missing/invalid coverage
    file → skip silently but still succeed (try/catch swallow).
12. **Success** → `{success:true, message:"Successfully renamed scenario to '<new>' in <basename>"}`.

### CLI registration
`update-scenario <feature> <old-name> <new-name>` — 3 positional args, no options.
On success: `output.log('✓ ' + message)`, exit 0. On failure: `output.error('Error:', error)`, exit 1.

## Rust mapping notes
- gherkin-0.16.0: `Feature.scenarios: Vec<Scenario>`, `Scenario.name: String`,
  `Scenario.keyword: String`, `Scenario.position: LineCol { line }`. 1-based line.
- Use `parse_feature_lenient` (io/gherkin.rs) for parity with @cucumber/gherkin tolerance.
- Coverage: `crate::types::coverage::CoverageFile` with `#[serde(flatten)] extra` preserves
  unknown fields. Rename `scenarios[i].name`, re-serialise pretty.
- Regex avoided per add_tag_to_feature pattern — hand-rolled prefix match for
  `Scenario:` / `Scenario Outline:` with leading-whitespace capture.
- Note: TS error envelope is INNER (Result<String> JSON with success:false), only arg-parse
  uses outer FspecCoreError — mirror list_scenario_tags.rs pattern.
