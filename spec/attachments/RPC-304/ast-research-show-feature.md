# RPC-304 — `show-feature` AST research

TS source-of-truth: `src/commands/show-feature.ts` + `src/commands/show-feature-help.ts`
+ supporting helpers in `src/utils/work-unit-tags.ts`.

## TypeScript command shape

```ts
interface ShowFeatureOptions {
  feature: string;                    // positional: bare name OR path
  format?: 'text' | 'json';           // default 'text'
  output?: string;                    // optional output file path
  cwd?: string;                       // defaults to process.cwd()
}

interface ShowFeatureResult {
  success: boolean;
  content?: string;
  format?: 'text' | 'json';
  validated?: boolean;
  error?: string;
  workUnits?: WorkUnitInfo[];
}
```

Commander.js registration (lines 197–207):
- Positional `<feature>` (required)
- `--format <format>` default `'text'`
- `--output <file>` (no default)
- NO `--workspace`/`--cwd` exposed.

`showFeatureCommand` exit-code contract:
- success + no `--output` → write `result.content` to stdout, exit 0.
- success + `--output` → write `✓ Feature content written to <file>` to stdout, exit 0.
- `result.success === false` → `output.error('Error:', result.error)` to stderr, exit 1.

## Resolution rules (lines 47–76)

1. If `feature.endsWith('.feature')` → treat as path joined to `cwd`. `access()`-check
   it; on ENOENT → `{success:false, error:'Feature file not found: <feature>'}`.
2. Otherwise glob `spec/features/**/*.feature` (cwd-relative), find first file whose
   basename (minus `.feature`) === `feature`. No match → same `'Feature file not
   found: ...'` error.
3. Read the resolved path as UTF-8.

## Gherkin parse (lines 81–94)

Uses `@cucumber/gherkin` `Parser` + `AstBuilder` + `GherkinClassicTokenMatcher`.
On throw → `{success:false, error:'Invalid Gherkin syntax: <error.message>'}`.

## Work-unit tag extraction (`src/utils/work-unit-tags.ts:46-125`)

Tag pattern: `^@([A-Z]{2,6}-\d+)$` (uppercase prefix 2-6 chars, dash, digits).
- Feature-level tags whose name matches the WU pattern → entry `{id, level:'feature',
  scenarios:[all scenarios WITHOUT their own WU tag]}`.
- Scenario-level WU tags → entry `{id, level:'scenario', scenarios:[that scenario]}`.
  Multiple scenarios tagged with same id accumulate; existing 'feature' level
  upgrades to 'scenario' when a scenario-level hit comes in.
- The `extractWorkUnitTags` function returns entries in the order: feature-level
  IDs first (declaration order in feature tag block), then scenario-level IDs
  (first-occurrence order across scenarios).

## Enrichment (lines 130–166)

`loadWorkUnitsData(cwd)` reads `<cwd>/spec/work-units.json`. On any failure
(`catch {}`) returns `null` and every WU enrichment maps to `title:'Unknown',
status:'unknown'`. On success, missing WU ids still map to `'Unknown'/'unknown'`.

## Output formatting

### Text path (lines 117–139)

```
<original file content>


Work Units:

  AUTH-001 (feature-level) - User Login
    login.feature:5 - Login with valid credentials
    login.feature:9 - Login with wrong password
```

When `workUnits.length === 0`:

```
<original file content>


Work Units: None
```

Note: blank line after file content via `'\n\n'` concat. `featurePath.split('/').pop()`
is the bare filename in scenario lines.

### JSON path (lines 104–116)

```js
JSON.stringify({...gherkinDocument, workUnits: workUnits.map(wu => ({
  id, title, status, level, scenarios
}))}, null, 2);
```

`gherkinDocument` is the raw @cucumber/gherkin AST containing `comments`, `feature`,
`feature.tags[].id` (UUIDs from `Messages.IdGenerator.uuid()`), `feature.tags[].location.{line,column}`,
`feature.children[].scenario.{id, tags, location, keyword, name, description, steps:[{id,location:{line,column},keyword,keywordType,text}], examples}`.

The Rust port intentionally does NOT replicate `@cucumber/gherkin` UUIDs/columns
byte-for-byte (the workspace already pins `gherkin = "0.16"` which produces a
DIFFERENT AST shape — no UUIDs, line-only locations, no `keywordType`).

## --output handling (lines 142–144)

`await writeFile(outputPath, outputContent, 'utf-8')`. Path is resolved against
process CWD (NOT project root) — TS calls `writeFile(outputPath, ...)` with the
raw string.

## Help (`src/commands/show-feature-help.ts`)

```
SHOW-FEATURE
Display the contents of a feature file with syntax highlighting

USAGE
  fspec show-feature <file>

ARGUMENTS
  <file> (required)
    Feature file path

OPTIONS
  No options available

EXAMPLES
  1. Display feature file
  $ fspec show-feature spec/features/login.feature
  Feature: User Login
  Scenario: Login with valid credentials
    Given I am on the login page...

RELATED COMMANDS
  fspec list-features
  fspec validate
```

⚠️ Help file does NOT advertise `--format` / `--output` even though Commander.js
registers them. Byte-for-byte parity therefore must omit them too.

## Rust port plan

- Reuse `io::feature_glob::glob_feature_files` for bare-name lookup.
- Use the workspace `gherkin = "0.16"` crate (already used by `list_scenario_tags`)
  for the JSON AST. Diverge from TS UUID shape — emit a structured "Rust port AST"
  shape and document the divergence in the feature file architecture notes.
  **OPEN QUESTION** for supervisor.
- Re-use the WU-tag inline scanner pattern (regex `^@([A-Z]{2,6}-\d+)$`).
- All recoverable errors live in the JSON envelope `{success, error, ...}`.
  Only `args_json` parse failures escalate via `FspecCoreError`.
