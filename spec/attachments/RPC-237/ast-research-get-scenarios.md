# AST Research — `get-scenarios` command (RPC-237)

## TS source surveyed
`src/commands/get-scenarios.ts` (6.2 KB) — small, single-purpose. Three exports:
`getScenarios` (core, line 32), `getScenariosCommand` (CLI handler, line 158),
`registerGetScenariosCommand` (Commander wiring, line 220).

### AST findings (structural)
- `interface ScenarioInfo { feature; name; line; tags? }` (line 10) and
  `interface GetScenariosResult { success; scenarios; totalCount; message; warnings?; error? }`
  (line 23) — declaration order is the serialization contract. Port mirrors with
  `#[derive(Serialize)]` structs (NOT `json!{}`, which alphabetizes via BTreeMap).
- Missing dir guard (lines 42–52): `access(featuresDir)` fail → `{success:false, error:'spec/features directory not found'}`. Port escalates via `FspecCoreError::Io` carrying that exact substring (mirrors show_acceptance_criteria.rs).
- Glob: `glob(['spec/features/**/*.feature'], {cwd, absolute:false})` (line 56) → reuse `io::feature_glob::glob_feature_files` (sorted, forward-slash rel paths).
- Empty file list (line 70) → `{success:true, totalCount:0, message:'No feature files found in spec/features/'}`.
- Parse: `@cucumber/gherkin` Parser; parse failure → `warnings.push('Skipping invalid file: <f>')` + continue (line 95) → reuse `io::gherkin::parse_feature_lenient`.
- Scenario filter (line 108): `child.scenario.keyword === 'Scenario'` only (excludes Scenario Outline).
- Tag union (line 113): `[...new Set([...featureTags, ...scenarioTags])]`; AND-logic `tags.every(tag => allTags.includes(tag))` (line 117). Gherkin keeps leading `@` on `t.name` here.
- Emitted record (line 123): `{feature, name, line, tags: scenarioTags.length>0 ? scenarioTags : undefined}` — scenario tags ONLY; feature tags used for matching but not stored.
- Message branches (lines 138–147): zero+tags / zero / found+tags / found, with `scenario`/`scenarios` pluralization on `totalCount === 1`.
- CLI handler (line 158): `--format json` prints `JSON.stringify(result.scenarios, null, 2)` (the **array only**, not the envelope); text prints message + blank line + scenarios grouped by feature as `  <line>: <name>[ [<tags>]]`.
- **Framing-A divergence**: `registerGetScenariosCommand` (line 220) registers ONLY `--tag` (repeatable) + `--format`; the help fixture (from `node dist/index.js get-scenarios --help`) shows a doc-only `--file`. Fixture is byte-parity canon; clap impl needs only `--tag`/`--format`.

## Rust port mapping
- `commands/get_scenarios.rs::run(args_json, project_root)` returns the full envelope; CLI bridge picks the rendered string per format. `format` arg is accepted for parity but unread by core (bridge renders) — annotated `#[allow(dead_code)]`.
- `Mode::GetScenarios { tag: Vec<String>, format: Option<String> }`.

## Conclusion
Behaviour fully characterised; gherkin-read parity well understood. Ready for
testing/implementing phases. See also `gherkin-port-notes.md`.
