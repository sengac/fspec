# RPC-282 — `remove-tag-from-scenario` — AST Research

## TS Source
`src/commands/remove-tag-from-scenario.ts` (226 LOC)
`src/commands/remove-tag-from-scenario-help.ts` (68 LOC)

## Observed TS behaviour

### Function signature
```ts
export async function removeTagFromScenario(
  featureFilePath: string,
  scenarioName: string,
  tags: string[],
  options: { cwd?: string } = {}
): Promise<{ success: boolean; valid: boolean; message?: string; error?: string }>
```

### Steps (line numbers refer to TS source)
1. **Path resolve** (26-27): `cwd = options.cwd || process.cwd()`; `filePath = join(cwd, featureFilePath)`.
2. **Read file** (30-42): on `ENOENT` → `{ success: false, valid: false, error: "File not found: <path>" }`. Other I/O errors are thrown.
3. **Parse Gherkin** (44-59): on parse error → `"Invalid Gherkin syntax: <message>"`.
4. **Missing feature** (61-67): `"File does not contain a valid Feature"`.
5. **Find scenario** (69-85): top-level `child.scenario.keyword === 'Scenario'` exact-name match. **Idempotent**: if not found → `{ success: true, valid: true, message: "Scenario '<name>' not found in <path> - no changes made" }`.
6. **Filter to existing tags** (87-100): `tagsToActuallyRemove = tags.filter(t => existingTags.includes(t))`. If none → `{ success: true, valid: true, message: "No changes made - none of the specified tags found on scenario '<name>'" }`.
7. **Locate scenario line** (102-120): scan for trimmed `"Scenario: <name>"`. Not found → `"Could not find Scenario line for "<name>""`.
8. **Walk lines and drop tag lines that belong to the target scenario** (122-162):
   - For each line `i < scenarioLineIndex`, check if `lines[i].trim().startsWith('@')`.
   - Determine if it belongs to the target scenario by scanning forward — the first `Scenario:` or `Feature:` we hit before the target marks a boundary.
   - If `belongsToTargetScenario && tagsToRemove.has(trimmed)` → skip (drop the whole tag line).
9. **Validate result** (165-175): re-parse; if fails set `valid=false`.
10. **Write** (178): direct overwrite.
11. **Return** (180-185): `{ success: true, valid, message: "Removed <tag1>, <tag2> from scenario '<name>'" }`.

### CLI surface
`fspec remove-tag-from-scenario <file> <scenario> <tags...>`
- No options.
- Variadic positional `<tags...>` — clap `Vec<String>` with `num_args = 1..`.
- Success stdout: `✓ <message>`.
- Failure stderr: `Error: <error>`; exit 1.

### Side effects
- Reads `spec/<file>`.
- Writes target feature file directly.

### Idempotent vs error behaviour summary
- File missing → ERROR (exit 1).
- Scenario missing → SUCCESS (idempotent).
- No tags matched → SUCCESS (idempotent).
- Tag found and removed → SUCCESS with message.

## Rust port plan

### Shared module reuse
- Reuses `crate::io::gherkin::parse_feature_lenient` for membership lookup.
- No new gherkin_tags helpers strictly needed (no tag-format validation in this command).

### Core impl
- Mirror TS line-walk filter exactly:
  - Collect line indices that should be dropped (set membership + boundary check).
  - Build new content by joining all OTHER lines.
- Write directly with `std::fs::write`.

### CLI bridge
- `pub struct CliArgs { pub file: String, pub scenario: String, pub tags: Vec<String> }`.
- Marshal as JSON `{file, scenario, tags}`.

### Two-front-doors
- Both paths call `commands::remove_tag_from_scenario::run(args_json, project_root)`.

## Risks / open questions
- The notes file mentions searching `feat.rules[].scenarios`. TS does NOT (filters `feature.children` for `keyword === 'Scenario'`). Following TS.
- TS `valid` flag is returned but not surfaced via CLI.
- Empty tags array would produce "No changes made" message — TS clap requires `<tags...>` (≥1).
