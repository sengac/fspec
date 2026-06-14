# AST Research — add-scenario (RPC-190)

## TS source: src/commands/add-scenario.ts

`addScenario(featureIdentifier, scenarioName, options{cwd, dryRun})`:
1. **Path resolution**:
   - ends with `.feature` → `cwd/<identifier>`.
   - starts with `spec/features/` → `cwd/<identifier>`.
   - else → `cwd/spec/features/<identifier>.feature`.
2. `readFile` — ENOENT → `{success:false, valid:false, error:'Feature file not found: <featurePath>', suggestion:"Use 'fspec create-feature' to create a new feature file"}`. Other errors rethrown.
3. Parse with @cucumber/gherkin. Parse failure → `{success:false, valid:false, error:'Feature file has invalid Gherkin syntax: <msg>', suggestion:"Run 'fspec validate <id>' to see syntax errors"}`.
4. `!gherkinDocument.feature` → `{success:false, valid:false, error:'Feature file does not contain a valid Feature', suggestion: ...}`.
5. **Duplicate check**: existing scenarios where `child.scenario.keyword === 'Scenario'`; if any has `name === scenarioName` → `warning = 'A scenario named "<name>" already exists in this feature'` (NON-fatal; still proceeds).
6. **Scenario template** (note leading + trailing newline):
```
\n  Scenario: <scenarioName>\n    Given [precondition]\n    When [action]\n    Then [expected outcome]\n
```
7. **Insertion point**: scan lines; first line whose trim starts with `Scenario Outline:` or `Scenario Template:` → insertIndex = that line. Else insertIndex = lines.length (append at end).
8. `newContent = lines.slice(0,insertIndex).join('\n') + scenarioTemplate + '\n' + lines.slice(insertIndex).join('\n')`.
9. Re-parse newContent → `valid` boolean (try/catch).
10. If `!dryRun` → writeFile.
11. Return `{success:true, valid, warning?}`.

### CLI command (addScenarioCommand)
- `!success` → `output.error('Error:', error)`, prints `Suggestion: <s>` if present, exit 1.
- warning → `output.log('⚠', warning)`.
- `output.log('✓ Added scenario "<scenarioName>"')`.
- exit 0; catch → `output.error('Error:', msg)` exit 1.

## Rust port plan
- New `add_scenario.rs` core: `run(args_json,&Path)`. Args `{feature, scenario, dryRun?}` (camelCase).
- Path resolution mirrors TS (endsWith/startsWith/else). project_root replaces cwd.
- Read file → not-found error; parse via `parse_feature_lenient` for validation + scenario-name list + duplicate detection.
  - Scenario name extraction: iterate `feature.scenarios`; keyword `Scenario` (gherkin crate exposes `keyword`). Need to filter keyword=="Scenario" only (exclude outlines). CONFIRM gherkin-0.16 Scenario.keyword field.
- Line-based insertion via split('\n')/slice/join exactly.
- Response JSON `{success, valid, warning?}` (+ message for CLI bridge to print). CLI prints ✓/⚠/Error+Suggestion.
- Two-front-doors: bridge marshals positional <feature> <scenario> + optional --dry-run → JSON.

## Open question
- gherkin-0.16 Scenario keyword access for "Scenario" vs "Scenario Outline". add_tag_to_feature uses feature.scenarios for tags. Need to confirm how to distinguish outline. For duplicate-check we compare `scenario.name`. For insertion we line-scan raw text (no AST needed). Duplicate detection: simplest is line-scan for `Scenario: <name>` too — but TS uses AST keyword filter. Will mirror with line-based name extraction to avoid keyword ambiguity; CONFIRM acceptable with supervisor.
