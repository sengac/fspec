# AST Research — `search-scenarios` (RPC-297)

## TS source of truth
- `src/commands/search-scenarios.ts`
- `src/commands/search-scenarios-help.ts` (rich help config — exists, NOT bare Commander.js)
- shared util: `src/utils/feature-parser.ts` (`parseAllFeatures`, `searchScenarios`)

## Behaviour (`searchScenarios(options)`)
1. `parseAllFeatures(cwd)` — globs `spec/features/*.feature` (NON-recursive: `glob(['*.feature'])`),
   parses each with `@cucumber/gherkin`, extracts scenarios + work-unit id from feature-level
   tag matching `/^@[A-Z]+-\d+$/` (first match wins, `@` stripped). Parse failures → skip file.
2. `searchScenariosUtil(parsedFeatures, query, regex, cwd)`:
   - if `regex`: compile `new RegExp(query, 'i')`; invalid → throw `Invalid regex pattern: "<q>". <msg>`.
   - Loads `spec/work-units.json` (best-effort; missing/invalid → `{}`) for work-unit TITLE search (BUG-059).
   - For each parsed feature: match against featureName OR featureDescription OR featureFilePath OR workUnitTitle.
     - If feature matches → emit ALL its scenarios.
     - else → emit only scenarios whose `scenario.name` matches.
   - `matchesQuery(text)`: regex → `pattern.test(text)`; literal → `text.toLowerCase().includes(query.toLowerCase())` (case-insensitive).
   - `workUnitId`: parsed.workUnitId || `'unknown'`.
3. Transform results: each `{ name: scenarioName (legacy dup), scenarioName, featureFilePath, workUnitId }`.
4. Returns `{ searchedFiles: parsedFeatures.length, scenarios, format: json?'json':'table', searchMode: regex?'regex':'literal' }`.

## CLI registration
- `.command('search-scenarios')`
- `.requiredOption('--query <pattern>', ...)` — REQUIRED
- `.option('--regex', ...)` — boolean flag
- `.option('--json', ...)` — boolean flag
- action: success → if `--json` print `JSON.stringify(result, null, 2)`; else green
  `✓ Found <N> scenarios matching "<query>"`. Error → `output.error('✗ Search failed:', msg)` + `process.exit(1)`.

## Rust wiring intent
- Reuse `io/feature_glob.rs` (NOTE: it walks `**` recursively + returns relative paths;
  TS uses NON-recursive `*.feature`. For parity, filter to top-level `spec/features/*.feature`
  OR document divergence. The featureFilePath emitted by TS is `join('spec','features', file)`
  i.e. `spec/features/<name>.feature` — flat). Decision: read `spec/features/` flat dir only.
- Reuse `io/gherkin.rs::parse_feature_lenient` for gherkin parse (feature name, description, scenarios, tags).
- Work-units title lookup: read `spec/work-units.json` best-effort.
- Output: dispatcher returns JSON envelope (2-indent) `{ searchedFiles, scenarios, format, searchMode }`;
  CLI bridge inspects `--json` to print envelope or green summary line.
- Help config exists → standard intercept arm.

## Edge cases
- Invalid regex → FspecCoreError (surfaced as Error). TS throws → caught → exit 1.
- Missing spec/features dir → TS glob returns [] → searchedFiles=0, empty scenarios (NOT an error).
- Case-insensitive literal match.
