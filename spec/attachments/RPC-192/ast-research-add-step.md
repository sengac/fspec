# AST Research — add-step (RPC-192)

## TS source: src/commands/add-step.ts

`addStep(featureIdentifier, scenarioName, stepType, stepText, options{cwd, dryRun})`:

1. **Validate step type**: normalize `stepType.toLowerCase()`. VALID = `['given','when','then','and','but']`. Invalid → `{success:false, valid:false, error:'Invalid step type: "<stepType>"', suggestion:'Valid step types are: given, when, then, and, but'}`.
2. **stepKeyword** = capitalize normalized: `Given`/`When`/`Then`/`And`/`But`.
3. **Path resolution** (same as add-scenario): endsWith `.feature`→cwd/id; startsWith `spec/features/`→cwd/id; else cwd/spec/features/<id>.feature.
4. `readFile` — ENOENT → `{success:false,valid:false,error:'Feature file not found: <path>', suggestion:"Use 'fspec create-feature'..."}`.
5. Parse gherkin. Failure → invalid-syntax error. `!feature` → does-not-contain-feature error.
6. **Find scenario** by keyword==='Scenario' & name===scenarioName. Not found → `{success:false,valid:false,error:'Scenario not found: "<name>"', suggestion:'Available scenarios: <comma-list or "none">'}`.
7. **scenarioLineIndex** = scenario.location.line - 1.
8. **stepIndentation**: default '    ' (4 spaces). If scenario has steps → indent of first step line (regex `^(\s+)`).
9. **Placeholder replacement**: placeholderMap {given:'[precondition]', when:'[action]', then:'[expected outcome]'}. If a step in this scenario has `text === placeholderText`, set placeholderStepIndex = step.location.line-1 (first match).
10. **If placeholder found**: replace that line with `${indent}${keyword} ${stepText}`.
11. **Else append**:
    - insertIndex = scenarioLineIndex+1 default.
    - If scenario has steps: lastStep = steps[last]; lastStepLineIndex = lastStep.location.line-1; insertIndex = lastStep.location.line (i.e. line AFTER last step keyword line). If next line (lastStepLineIndex+1) trim starts with `|` or `"""` → insertIndex = lastStepLineIndex+1 (insert before the table/docstring).
    - Else (no steps): scan from scenarioLineIndex+1; first line whose trim starts with `Scenario:`/`Scenario Outline:` or is empty → insertIndex=i, break; otherwise insertIndex=i+1.
    - newStep = `${indent}${keyword} ${stepText}`; splice into lines at insertIndex.
12. Re-parse newContent → valid bool.
13. If !dryRun → writeFile.
14. Return `{success:true, valid}`.

### CLI command (addStepCommand)
- !success → `output.error('Error:', error)`, prints `Suggestion: <s>`, exit 1.
- success → `output.log(chalk.green('✓ Added <stepType> step to scenario "<name>"'))`, exit 0.
- catch → error exit 1.

## Rust port plan
- New `add_step.rs` core: `run(args_json,&Path)`. Args `{feature, scenario, type, text, dryRun?}` (camelCase; `type` is reserved → use `#[serde(rename="type")] step_type`).
- Validate type set; build keyword via capitalize.
- Path resolution mirror; read → not-found.
- Parse via `parse_feature_lenient`. Find scenario: `feature.scenarios.iter().find(|s| s.keyword.trim()=="Scenario" && s.name==scenario)`.
  - gherkin-0.16 exposes `s.position.line`, `s.steps[].position.line`, `st.keyword`, `st.value`.
- Step text comparison: `st.value` (== TS step.text). keyword via `st.keyword.trim()`.
- Indentation: read raw line `lines[firstStep.position.line-1]`, capture leading whitespace.
- Placeholder map for given/when/then. Replace or append using `.position.line` exactly like TS `.location.line`.
- Re-parse for valid; write unless dry_run.
- Response `{success, valid}` (+ stepType/scenario for CLI message). CLI prints ✓/Error+Suggestion.
- Two-front-doors: bridge marshals positional <feature> <scenario> <type> <text> + optional --dry-run → JSON.

## Confirmed
- gherkin-0.16 Scenario.position.line, Step.position.line, Step.value, Step.keyword, Scenario.keyword all available (used in show_acceptance_criteria.rs / show_feature.rs).
