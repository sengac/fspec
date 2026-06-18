# RPC-295 — `review` — AST Research (discovery grounding)

Grounds the Rust port of `src/commands/review.ts` against existing Rust infra.
All findings gathered via AstGrep / Grep over `codelet/fspec-core/`.

## 1. Signature precedent — show_work_unit (the closest sibling)

`codelet/fspec-core/src/commands/show_work_unit.rs:68`
```
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>
```
CONFIRMS ruling 0: review uses the 2-arg `run(args_json, project_root)` shape,
args `{ workUnitId: string }`. Reads `spec/work-units.json` via bare
`std::fs::read_to_string` (no auto-create); missing unit →
`"Work unit '<id>' does not exist"`.

## 2. Linked-feature lookup — RE-IMPLEMENT locally (ruling 2)

`show_work_unit.rs:440 fn scan_linked_features(project_root, work_unit_id) -> Vec<LinkedFeature>`
is PRIVATE. The review port re-implements the equivalent lookup locally:
- `crate::io::feature_glob::glob_feature_files(project_root) -> Result<Vec<String>, FspecCoreError>`
  (`io/feature_glob.rs:33`) enumerates `spec/features/*.feature`.
- `crate::io::gherkin::parse_feature_lenient(content) -> Result<Feature, ParseError>`
  (`io/gherkin.rs:62`) parses each file; matches `@PREFIX-NNN` feature/scenario tags
  via the same `extract_work_unit_id` rule (uppercase prefix 2-6 chars, digit suffix).
TS review only needs `linkedFeatures[0].file`, so the local lookup can return the
first matching feature path (and degrade to "no linked feature" on any error,
matching the TS bare try/catch).

## 3. Gherkin parse-failure warning (rule [2])

TS wraps `parser.parse` in try/catch and, on failure, pushes the warning
`"Invalid Gherkin syntax in feature file"`. The Rust port mirrors this by treating
`parse_feature_lenient` Err on the FIRST linked feature as that warning (does not
escalate).

## 4. Agent runtime — INLINE a private copy (ruling 1)

`src/utils/agentRuntimeConfig.ts`:
- `getAgentConfig(cwd)`: priority `FSPEC_AGENT` env > `spec/fspec-config.json` `{agent}` > safe default
  (`{ category: 'cli', supportsSystemReminders: false }`).
- `formatAgentOutput(agent, msg)`:
  - `supportsSystemReminders` → `"<system-reminder>\n{msg}\n</system-reminder>"`
  - `category ∈ {ide, extension}` → `"**⚠️ IMPORTANT:** {msg}"`
  - else (cli/default) → `"**IMPORTANT:** {msg}"`

Precedent for inlining: `init.rs:64-118` already inlines the full `AGENT_REGISTRY`
(19 agents) with `id, supports_system_reminders, category {Ide,Cli,Extension}` and a
`get_agent_by_id(id) -> Option<&'static Agent>` lookup. review only needs
`supports_system_reminders` + `category`, so a trimmed private table (or reuse of the
same shape) suffices. Future-consolidation note on RPC-295 tracks extracting a shared
`agent_runtime` module.

## 5. Coverage + coding-standards scan (rules [4][5][6])

review reads `<feature>.feature.coverage` JSON directly via `std::fs` + `serde_json`
(no shared coverage type required for read-only projection):
- `stats { totalScenarios, coveredScenarios, coveragePercent }`
- `scenarios[] { name, testMappings[] { file, lines } }`
TS-verbatim regexes scanned over each `mapping.file` test file content (ruling 3):
`": any"`, `/require\(/`, `/import .* from ['"].*\.(ts|js)['"]/`.

## 6. Error type

`FspecCoreError` (`crate::error`): `InvalidArgs { command, reason }` for the
not-found / missing-arg paths, matching show_work_unit. The dispatcher wraps
`Ok(String)` into `DispatchResult.data` verbatim (the report text blob).

## 7. Architecture constraint

All I/O is BLOCKING `std::fs` (read work-units.json, feature files, coverage files,
test files) — runs under `poll_sync_future` (polls once); NO real async `.await`.
