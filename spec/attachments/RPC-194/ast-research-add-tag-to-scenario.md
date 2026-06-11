# RPC-194 — `add-tag-to-scenario` — AST Research

## TS Source
`src/commands/add-tag-to-scenario.ts` (282 LOC)
`src/commands/add-tag-to-scenario-help.ts` (56 LOC)

## Observed TS behaviour

### Function signature
```ts
export async function addTagToScenario(
  featureFilePath: string,
  scenarioName: string,
  tags: string[],
  options: { cwd?: string; validateRegistry?: boolean } = {}
): Promise<{ success: boolean; valid: boolean; message?: string; error?: string }>
```

### Steps (line numbers refer to TS source)
1. **Path resolve** (28-29): `cwd = options.cwd || process.cwd()`; `filePath = join(cwd, featureFilePath)`.
2. **Read file** (32-44): on `ENOENT` → `{ success: false, valid: false, error: "File not found: <featureFilePath>" }`. Other I/O errors are thrown.
3. **Validate each tag format** (47-66):
   - Must start with `@` → error `"Invalid tag format. Tags must start with @"`.
   - Allow work-unit tag pattern (`^@[A-Z]{2,6}-\d+$`) OR regular tag pattern (`^@[a-z0-9-#]+$`).
   - Otherwise: error `"Invalid tag format. Regular tags must use lowercase-with-hyphens, work unit tags must match @[A-Z]{2,6}-\\d+ (e.g., @AUTH-001)"`.
4. **Parse Gherkin** (68-83): `@cucumber/gherkin` Parser. On parse error → `"Invalid Gherkin syntax: <message>"`.
5. **Missing Feature** (85-91): `"File does not contain a valid Feature"`.
6. **Find scenario by name** (94-108): filter children where `child.scenario && child.scenario.keyword === 'Scenario'`, then exact-match `child.scenario.name === scenarioName`. NOT found → `"Scenario '<name>' not found in <path>"`. Only top-level `Scenario:` is searched (NOT background, NOT scenario outline, NOT rule-nested). Notes file says "Must search both `feat.scenarios` and `feat.rules[].scenarios`" but TS does NOT — the actual TS code filters `gherkinDocument.feature.children`. We'll match TS.
7. **Duplicate check** (110-122): existingTags = scenario.tags.map(t => t.name). For each new tag in `existingTags` → error `"Tag <tag> already exists on this scenario"`.
8. **Registry validation** (124-153, only when `--validate-registry` set):
   - Reads `spec/tags.json`, builds set of all `category.tags[].name`.
   - For each tag not in set → error `"Tag <tag> is not registered in spec/tags.json"`.
   - If file read/parse fails: `"Failed to validate against registry: <io error message>"`.
9. **Locate scenario line in text** (156-173): scan `lines = content.split('\n')` for first line whose trim equals `"Scenario: <name>"`. If not found → error `"Could not find Scenario line for "<name>""`.
10. **Compute insert index** (175-204):
    - Default insertIndex = scenarioLineIndex.
    - Walk upward; if non-tag, non-empty line found → `insertIndex = i + 1`; if i == 0 or empty line → keep `scenarioLineIndex`.
    - If existing tags (existingTags.length > 0): walk upward looking for first `@` line; `insertIndex = i + 1` (insert after last existing tag).
11. **Indentation** (191-193): `scenarioLine.match(/^(\s*)/)?.[1] || '  '`.
12. **Insert tags** (206-210): `lines.splice(insertIndex, 0, ...tagLines)` where each tagLine = `${indent}${tag}`.
13. **Validate result** (212-221): re-parse newContent; if parse fails set `valid=false` (but still proceed).
14. **Write atomically** (223): `writeFile(filePath, newContent, 'utf-8')`. NOT atomic, NOT locked — direct overwrite.
15. **Return** (226-231): `{ success: true, valid: <bool>, message: "Added <tag1>, <tag2> to scenario '<name>'" }`.

### CLI surface
`fspec add-tag-to-scenario <file> <scenario> <tags...> [--validate-registry]`
- Variadic positional `<tags...>` — clap `Vec<String>` with `num_args = 1..`.
- Option `--validate-registry` (boolean).
- Success stdout: `✓ <message>` via `output.log`.
- Failure stderr: `Error: <error>` via `output.error('Error:', result.error)`; exit 1.

### Side effects
- Reads `spec/<file>` (relative to cwd).
- Optionally reads `spec/tags.json`.
- Writes target feature file directly (no transactions/locks).

## Rust port plan

### New shared module: `codelet/fspec-core/src/gherkin_tags.rs`
Helpers:
- `pub fn is_work_unit_tag(s: &str) -> bool` — matches `^@[A-Z]{2,6}-\d+$`.
- `pub fn is_regular_tag(s: &str) -> bool` — matches `^@[a-z0-9-#]+$`.

### Core impl
- Use `gherkin::Feature::parse` via `parse_feature_lenient` (already exists) ONLY for tag membership + scenario existence check. Reuse the same lenient parser to mirror TS Gherkin tolerance.
- Text mutation done on raw lines (NOT via re-emit) so we preserve formatting — matches TS line splice approach 1:1.
- File writes use direct `std::fs::write` (parity — TS uses `writeFile`).

### CLI bridge
- `pub struct CliArgs { pub file: String, pub scenario: String, pub tags: Vec<String>, pub validate_registry: bool }`.
- Marshal as JSON `{file, scenario, tags, validateRegistry}` (only include `validateRegistry: true` when set — bool with serde default `false`).

### Help fixture
- Capture from `node dist/index.js add-tag-to-scenario --help` (see playbook).

### Two-front-doors
- Same `commands::add_tag_to_scenario::run(args_json, project_root)` serves dispatcher AND CLI bridge.

## Risks / open questions
- The notes file says to search rules[].scenarios. TS does NOT. Following TS as canon.
- TS `valid: false` outcome is rare (re-parse failure post-write). We will preserve the flag in JSON but it is not observable from CLI surface.
- Returns object envelope (success, valid, message, error) — kept verbatim.
