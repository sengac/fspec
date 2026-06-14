# AST Research — update-step (RPC-315)

## TS source: src/commands/update-step.ts
Line-based update of a step's text and/or keyword within a named scenario.

### Behaviours (rule candidates)
1. **At-least-one-update guard**: if neither `text` nor `keyword` → `{success:false,
   error:"No updates specified. Use --text and/or --keyword"}`.
2. **Feature-path resolution** (same 3-way as update-scenario).
3. **File-not-found** (ENOENT) → `{success:false, error:"Feature file not found: <abs>"}`.
4. **Invalid Gherkin** → `{success:false, error:"Invalid Gherkin syntax: <msg>"}`.
5. **No Feature element** → `{success:false, error:"Feature file does not contain a valid Feature"}`.
6. **Scenario not found** → `{success:false, error:"Scenario '<scenario>' not found in feature file"}`.
7. **Step matching**: among `scenario.steps`, match where ANY of:
   - `step.text === currentStep`
   - `(step.keyword + step.text).trim() === currentStep.trim()`
   - `(step.keyword.trim() + ' ' + step.text) === currentStep.trim()`
   If none → `{success:false, error:"Step '<currentStep>' not found in scenario '<scenario>'"}`.
8. **Locate step line** via `step.location.line` (1-based) → 0-indexed.
   Regex `/^(\s*)(Given|When|Then|And|But)\s+(.+)$/` to capture indent/keyword/text.
   If fails → `{success:false, error:"Could not parse step line"}`.
9. **New keyword** = `keyword || currentKeyword`.
10. **New text**: if `text` provided — if text itself begins with a keyword
    (`/^(?:Given|When|Then|And|But)\s+(.+)$/`), use captured remainder; else use text as-is.
    If no text → keep currentText.
11. **Replace** line with `${indent}${newKeyword} ${newText}`.
12. **Re-validate** Gherkin; if invalid → `{success:false,
    error:"Update would result in invalid Gherkin: <msg>"}`.
13. **Write** updated content (line-based split/join). No coverage update for update-step.
14. **Success** → `{success:true, message:"Successfully updated step in scenario '<scenario>' in <basename>"}`.

### CLI registration
`update-step <feature> <scenario> <current-step> [--text <text>] [--keyword <keyword>]`.
3 positionals + 2 options. Success: `output.log('✓ ' + message)`, exit 0; failure exit 1.

## Rust mapping notes
- gherkin-0.16.0: `Scenario.steps: Vec<Step>`, `Step.keyword: String` (includes trailing space,
  e.g. "Given "), `Step.value: String` (text), `Step.position: LineCol`.
  NOTE: TS `step.keyword` from @cucumber/gherkin also includes trailing space — match handling.
- Use `parse_feature_lenient`.
- Hand-roll keyword prefix matching (no regex dep) — mirror add_tag_to_feature is_*_tag style.
- Inner JSON error envelope like list_scenario_tags.rs.
- No coverage side-effect (unlike update-scenario).
