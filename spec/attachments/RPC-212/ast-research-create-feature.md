# AST Research — create-feature (RPC-212)

## TS source: src/commands/create-feature.ts

`createFeature(name, cwd=process.cwd())`:
1. `featuresDir = cwd/spec/features`; `fileName = toKebabCase(name) + '.feature'`; `filePath = featuresDir/fileName`.
2. **File-exists check** — `access(filePath)`:
   - exists → throw `File already exists: spec/features/<fileName>\nSuggestion: Use a different name or delete the existing file`.
   - EACCES → throw `Permission denied: Cannot access spec/features/<fileName>\nSuggestion: Check file permissions for the spec/features directory`.
   - other (non-ENOENT) → throw `Failed to check if file exists: <msg>\nSuggestion: ...`.
   - ENOENT → proceed.
3. `mkdir(featuresDir, {recursive})` — EACCES → `Permission denied: Cannot create directory spec/features/...`; other → `Failed to create directory: <msg>`.
4. `content = generateFeatureTemplate(name)` (src/utils/templates.ts).
5. `writeFile(filePath, content)` — EACCES/ENOSPC/other → specific errors.
6. `createCoverageFile(filePath)` — graceful: catch → coverageFile {created:false, status:'error', message:'Warning: Failed to create coverage file: <msg>'}.
7. Re-read file, `detectPrefill(content)`.
8. `fileNamingReminder = getFileNamingReminder(toKebabCase(name))`.
9. Returns `{filePath, prefillDetection{hasPrefill,matches,systemReminder?}, coverageFile{created,path?,status,message}, fileNamingReminder?}`.

### generateFeatureTemplate(name) — src/utils/templates.ts
```
@critical @component @feature-group
Feature: <name>

  """
  Architecture notes:
  - TODO: Add key architectural decisions
  - TODO: Add dependencies and integrations
  - TODO: Add critical implementation requirements
  """

  Background: User Story
    As a [role]
    I want to [action]
    So that [benefit]

  Scenario: [Scenario name]
    Given [precondition]
    When [action]
    Then [expected outcome]
```
(Note: ends with trailing newline.)

### toKebabCase — src/utils/file-helpers.ts
```
str.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')
```

### createCoverageFile — src/utils/coverage-file.ts
- coverageFilePath = `${featureFilePath}.coverage`.
- If not exists (ENOENT) → writeCoverageFile → status 'created', message `✓ Created <fileName>` (basename).
- writeCoverageFile parses feature, extracts scenario names, builds `{scenarios:[{name,testMappings:[]}...], stats:{totalScenarios,coveredScenarios:0,coveragePercent:0,testFiles:[],implFiles:[],totalLinesCovered:0}}`, writes `JSON.stringify(coverage, null, 2)`.
- For NEW file (template) the lone scenario name is `[Scenario name]`.

### detectPrefill — src/utils/prefill-detection.ts
Template contains: `[role]`, `[action]`, `[benefit]`, `[precondition]`, `[expected outcome]`, `[Scenario name]` (matched as `[scenario name]` case-insensitive), and `TODO:` (x3). The `@component`/`@feature-group` multiline patterns also match the `@critical @component @feature-group` tag line. So `hasPrefill=true` with a systemReminder.

### getFileNamingReminder — src/utils/system-reminder.ts
- `isTaskBasedNaming(name)`: true if name matches `^implement-|^add-|^create-|^fix-|^build-|^setup-|^update-` (case-insensitive) OR `^[A-Z]+-\d+$`.
- When true & reminders enabled → returns wrapped reminder. Else null → omitted.

### CLI command (createFeatureCommand)
- Prints `✓ Created spec/features/<file>` then `  Edit the file to add your scenarios`.
- Prints coverage message (status-dependent).
- Prints fileNamingReminder ("\n" + reminder) if present.
- Prints prefill systemReminder ("\n" + reminder) if hasPrefill.
- exit 0; on error `output.error('Error:', msg)` + exit 1.

## Rust port plan
- New `create_feature.rs` core: `run(args_json,&Path)`. Args: `{name}`.
- Helpers: `to_kebab_case`, `feature_template`, write coverage via `crate::types::coverage::{CoverageFile, CoverageScenario, CoverageStats, calculate_stats}` + parse scenario names via `parse_feature_lenient` (or simple line scan for `Scenario:`).
- Reuse prefill detection? No Rust equivalent exists yet → port a minimal `detect_prefill` inline OR ask supervisor. For PHASE A, plan: port prefill + file-naming reminder logic inline in create_feature.rs (no shared io change needed). NEEDS CONFIRM: is there an existing prefill util in fspec-core? (grep shows none.)
- Response JSON envelope mirrors TS result shape.
