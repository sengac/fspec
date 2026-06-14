# AST Research — `validate-tags` (RPC-324)

## TS source files
- `src/commands/validate-tags.ts` — entry + `validateTags()` programmatic API + `validateTagsCommand()` CLI wrapper + `registerValidateTagsCommand()`.
- `src/commands/validate-tags-registry.ts` — `loadTagRegistry(cwd)` → `{ validTags:Set, requiredCategories:{component[],featureGroup[]} }` via `ensureTagsFile`.
- `src/commands/validate-tags-file.ts` — `validateFileTags(file, registry, workUnitsData, cwd)` → per-file `{ file, valid, errors[] }`. Parses gherkin, extracts feature + scenario tags.
- `src/commands/validate-tags-output.ts` — `renderValidateTagsOutput()` printing logic.
- `src/commands/validate-tags-help.ts` — help config.
- `src/utils/work-unit-tags.ts` — `isWorkUnitTag` (`/^@([A-Z]{2,6}-\d+)$/`), `looksLikeWorkUnitTag` (`/^@([a-zA-Z]{2,6}-\d+)$/`), `extractWorkUnitId`, `loadWorkUnitsData(cwd)` (returns null on any error — does NOT auto-create).

## Behaviour summary
### loadTagRegistry
- `ensureTagsFile(cwd)` → auto-creates `spec/tags.json` if missing (9 categories).
- validTags = flat set of every `tag.name` across all categories.
- requiredCategories.component = tag names where category.name === 'Component Tags'.
- requiredCategories.featureGroup = tag names where category.name === 'Feature Group Tags'.

### loadWorkUnitsData
- Reads `spec/work-units.json`; returns `null` on ANY error (ENOENT or parse). Does NOT auto-create.

### File selection
- `options.file` set → `[options.file]`.
- else → glob `spec/features/**/*.feature` (relative paths).
- Empty file set → `{ results:[], validCount:0, invalidCount:0 }`.

### validateFileTags (per file)
- Reads file; gherkin-parses. Parse failure → returns `valid:true` (skip). No `feature` → returns `valid:true`.
- featureTags = feature.tags[].name; scenarioTags = all scenario.tags[].name across children.
- **validateUnregisteredFeatureTags**: for each feature tag NOT in validTags:
  - isWorkUnitTag → reportWorkUnitTag (see below)
  - else looksLikeWorkUnitTag → error "Invalid work unit tag format: <tag>" + suggestion (pattern)
  - else tag === '@component' || '@feature-group' → error "Placeholder tag: <tag>" + suggestion "Replace ... with actual tags from tags.json"
  - else → error "Unregistered tag: <tag> in <file>" + suggestion "Register this tag in spec/tags.json or use 'fspec register-tag'"
- **validateScenarioTags**:
  - scenario-level work-unit tags (isWorkUnitTag) → error "Work unit ID tag <tag> must be at feature level, not scenario level" + suggestion (BUG-005).
  - unregistered scenario tags (excluding isWorkUnitTag already handled): looksLikeWorkUnitTag → "Invalid work unit tag format"; else "Unregistered tag: <tag> in <file>".
- **validateRequiredCategoryTags** (uses feature tags only):
  - no component tag AND no '@component' placeholder → error "Missing required component tag" + suggestion "Add one of: <list>".
  - no feature-group tag AND no '@feature-group' placeholder → "Missing required feature-group tag".
- reportWorkUnitTag: extractWorkUnitId → if null → "Invalid work unit tag format"; if workUnitsData null → "Work unit <tag> found but spec/work-units.json does not exist"; if id not in workUnits → "Work unit <tag> not found in spec/work-units.json".
- On read/parse exception inside try → valid=false, errors push `{tag:'', message}`.

### validateTags aggregate
- results = Promise.all over files.
- validCount = count valid; invalidCount = total - valid.

### validateTagsCommand (CLI)
- calls validateTags({file}); renderValidateTagsOutput; exit 1 if invalidCount>0 else 0; catch → output.error('Error:', msg) exit 2.

### renderValidateTagsOutput rules
- summaryOnly = options.summary===true. verbose = options.verbose && !summaryOnly.
- If !summaryOnly: for each result: valid+verbose → "✓ All tags in <file> are registered"; invalid → "✗ <file> has tag violations:" then "  <error.message>" and (if suggestion) "  Suggestion: <suggestion>".
- shouldPrintSummary = summaryOnly || results.length>1.
- summary lines: "✓ <validCount> files passed"; if invalidCount>0 also "✗ <invalidCount> files have tag violations".
- Single file, no flags, no failures → NO output at all.

## Help (validate-tags-help.ts)
- name validate-tags; description "Validate that all tags in feature files are registered in spec/tags.json and enforce tag placement rules"; usage 'fspec validate-tags'.
- whenToUse, one example, 2 commonErrors, relatedCommands [register-tag, list-tags, check], 4 notes.
- HAS a custom -help.ts → needs help config module.

## Flags (Commander registration)
- argument `[file]` (optional positional).
- `--verbose`, `--summary` (booleans, no short).

## Exit codes: 0 valid, 1 invalid, 2 error.

## Rust wiring intent
- Reuse `ensure_tags_file` (RPC-251) for registry, NEW read-only `load_work_units_or_null` helper (loadWorkUnitsData returns null on any error — different from read_work_units_or_empty which returns empty struct; null matters because reportWorkUnitTag branches on null). REQUEST to supervisor: add `read_work_units_or_null(cwd) -> Result<Option<WorkUnitsData>>` to io/ensure.rs (or model null as Option in command).
- Reuse `glob_feature_files` (RPC-245) for the no-file glob path. NOTE: glob_feature_files escalates DirectoryNotFound when spec/features missing — TS tinyglobby returns [] silently. Need parity: empty file set, not error. REQUEST: confirm whether to add a non-throwing glob variant or handle in command.
- Gherkin parsing: need a tag extractor. Check whether codelet has a gherkin parser (list-feature-tags RPC-244 used embedded gherkin parsing). Reuse that.
- Work-unit tag regex helpers ported into command or a shared util.
