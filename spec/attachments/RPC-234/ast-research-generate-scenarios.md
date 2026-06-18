# RPC-234 — `generate-scenarios` — Port Research (Phase A)

TS source of truth: `src/commands/generate-scenarios.ts` (+ `generate-scenarios-help.ts`).
Reference port pattern: `list-prefixes` (RPC-248); closest complex sibling: `reverse` (RPC-294).

## What the command DOES (behavioural contract)

`generate-scenarios <workUnitId> [--feature=<name>] [--ignore-possible-duplicates]`

Despite the name, the command **does NOT emit scenarios**. It creates a
*context-only* `.feature` scaffold (tag line + `Feature:` + architecture
docstring + `# EXAMPLE MAPPING CONTEXT` comment block + `Background:` user
story) with **ZERO `Scenario:` blocks**, then returns system-reminders telling
the AI agent to hand-write the scenarios. `scenariosCount` is always `0`.
(Note: `generate-scenarios-help.ts` examples claim "Generated 3 scenarios" —
this is stale help-doc text; the actual runtime produces a context-only file.
The help **fixture** captured from `--help` is still byte-parity canon.)

### Control flow (order matters — first failing gate throws)
1. `cwd = options.cwd || process.cwd()` → in Rust this is `project_root` (passed in, never `env::current_dir()` in core).
2. `ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json` if missing.
3. Work unit missing → throw `Work unit '<id>' does not exist`.
4. **Unanswered-questions gate**: count `questions` where `!deleted && !selected`. If `>0` → throw
   `Cannot generate scenarios: N unanswered question(s) found.\n\n<reminder>\n\nAnswer questions with 'fspec answer-question <id> <index>' before generating.`
   (singular/plural "question"/"questions").
5. **Empty Example Mapping gate**: `hasRules = rules.length>0`, `hasExamples = examples.length>0`. If `!hasRules && !hasExamples` → throw
   `Cannot generate scenarios: No Example Mapping data found.\n\n<reminder>\n\nComplete Example Mapping before generating scenarios.`
6. **No active examples gate**: `activeExamples = examples.filter(!deleted)`. If empty → throw
   `Work unit <id> has no examples to generate scenarios from`.
7. **Duplicate detection** (`scanExistingFeatures`): glob `spec/features/**/*.feature`, parse each with gherkin, collect `{name, steps}` per scenario. For each active example: `extractStepsFromExample(text)` → build target `{name, steps}` → `findMatchingScenarios(target, allFeatures, 0.7)`. Collect matches per example index.
   - If matches found AND `!ignorePossibleDuplicates` → throw
     `Cannot generate scenarios: N duplicate scenarios detected above threshold.\n\n<system-reminder>DUPLICATE SCENARIOS DETECTED ...</system-reminder>\n\nInvestigate feature files or use --ignore-possible-duplicates to proceed.`
   - If matches found AND `ignorePossibleDuplicates` → log warnings (`⚠ Detected potential refactor (ignored)...`) to stdout, continue.
8. **Feature path**: if `--feature` → strip trailing `.feature`, `join(cwd, "spec/features", name+".feature")`. Else kebab-case `workUnit.title` (lowercase, `[^a-z0-9]+`→`-`, trim leading/trailing `-`); if no title → throw `Cannot determine feature file name. Work unit <id> has no title.\nSuggestion: Use --feature flag ...`.
9. `mkdir -p dirname(featureFile)`.
10. **File-exists gate**: if file already exists → throw `Feature file <path> already exists.\ngenerate-scenarios creates context-only files ...`.
11. Build content:
    ```
    @<id>
    Feature: <title>

    <architectureDocstring>

    <commentBlock>

    <backgroundSection>
    ```
    - `backgroundSection` from `userStory{role,action,benefit}` if present, else `[role]/[action]/[benefit]` placeholders.
    - `architectureDocstring` from `architectureNotes` (active, !deleted) grouped by prefix via `categorizeArchitectureNotes` (known prefixes: Dependency/Dependencies→Dependency, Performance, Refactoring/Refactor→Refactoring, Security, UI/UX, Implementation; rest→General). General notes printed bare first, categorised notes under `<Category>:` headers with prefix stripped + `- ` bullet. If no notes → placeholder docstring with `TODO:` lines.
    - `commentBlock` = `generateExampleMappingComments`: bordered `# EXAMPLE MAPPING CONTEXT` block with `# BUSINESS RULES:`, `# EXAMPLES:`, `# QUESTIONS (ANSWERED):` (Q/A, `@human:` prefix stripped), `# ASSUMPTIONS:`.
12. `writeFile`.
13. `detectPrefill(finalContent)` — see prefill-detection.ts patterns.
14. Build `systemReminders[]`: (a) `CONTEXT-ONLY FEATURE FILE CREATED` reminder (lists examples with YES/NO experience check), (b) `getPostGenerationReminder`, (c) prefill reminder if any.
15. Return `{success:true, featureFile, scenariosCount:0, systemReminders, systemReminder:consolidateReminders(...), detectedMatches?, updatedFeatures?, createdFeature (path minus cwd/ and spec/features/), coverageRegenerated:false}`.

### CLI surface (`registerGenerateScenariosCommand`)
- positional `<workUnitId>` (required)
- `--feature <name>` (optional)
- `--ignore-possible-duplicates` (boolean flag)
- Success stdout: `✓ Created context-only feature file: <path>` then `  Contains example mapping context as comments (NO scenarios yet)` then (if any) the consolidated `systemReminder`.
- Error: `output.error(chalk.red('✗ Failed to generate scenarios:'), msg)` to stderr, `process.exit(1)`.

## Dependencies (TS utils → Rust port targets)
| TS util | Role | Rust status |
|---|---|---|
| `utils/ensure-files.ensureWorkUnitsFile` | auto-create work-units.json | EXISTS — `io::ensure` |
| `tinyglobby.glob('spec/features/**/*.feature')` | enumerate features | EXISTS — `io::feature_glob::glob_feature_files` |
| `@cucumber/gherkin` parse | extract scenario name+steps | EXISTS — `io::gherkin::parse_feature_lenient` (gherkin crate) |
| `utils/step-extraction.extractStepsFromExample` | heuristic G/W/T from example text | **NEW — port (4 regex patterns + prefill fallback)** |
| `utils/scenario-similarity.findMatchingScenarios` | adaptive-threshold matcher | **NEW — port** |
| `utils/similarity-algorithms.hybridSimilarity` | 5-algorithm weighted combo (JaroWinkler, TokenSet, Trigram, Jaccard, GherkinStructural) | **NEW — substantial port (~430 LOC)** |
| `utils/prefill-detection.detectPrefill` | placeholder detection + reminder | **NEW — port (regex table)** |
| `utils/system-reminder.{getUnansweredQuestionsReminder,getEmptyExampleMappingReminder,getPostGenerationReminder,consolidateReminders}` | reminder text | **NEW — port (verbatim strings)** |

Plan: keep all NEW helper logic as **private modules inside my owned command file**
`codelet/fspec-core/src/commands/generate_scenarios.rs` (or sibling files I own) to
avoid touching shared `types/mod.rs` / new shared dirs. WorkUnit example-mapping
fields (`userStory`, `rules`, `examples`, `questions`, `assumptions`,
`architectureNotes`) live in `WorkUnit.extra` (only `id/type/title/status/...`
are typed) — read them out of `extra` as `serde_json::Value`, mirroring the
TS untyped access.

## Async / child-process / network assessment
**NONE.** The command is pure blocking `fs` (`readFile`/`writeFile`/`mkdir`/`existsSync`)
+ glob + in-process gherkin parsing + string/regex CPU work. No network, no
spawned children, no real `.await` on tokio resources. **Fully compatible with
`poll_sync_future`** (resolves on first poll). The only side-effect divergence:
TS `output.log` warnings in the `--ignore-possible-duplicates` branch are
printed directly; the Rust core returns a String, so those warnings must be
folded into the returned rendered output instead of a separate stdout write.

## Shared-file changes to REQUEST from supervisor (Phase C)
1. `dispatch.rs`: change the `generate-scenarios` arm to call
   `commands::generate_scenarios::run(args_json, project_root).await` — the
   ported signature gains `project_root` (parity: cwd drives glob + write path;
   never `env::current_dir()` in core). Remove its `run_stub` arm.
2. `canonical.rs`: add `"generate-scenarios"` to `PORTED_COMMANDS`.
3. `help/configs/mod.rs`: register `pub mod generate_scenarios;`.
4. `main.rs`: add `mod generate_scenarios;`, a `Mode::GenerateScenarios { work_unit_id, feature, ignore_possible_duplicates }` clap variant (positional `<workUnitId>`, `--feature <name>`, `--ignore-possible-duplicates` bool), a `forward!` arm, and a `--help` intercept arm calling `configs::generate_scenarios::CONFIG`.
5. `commands/mod.rs`: stub module `generate_scenarios` already registered — **no change** expected (verify only).
6. `types/mod.rs`: **no new shared type module** anticipated (reuse `WorkUnit`, read example-mapping fields from `extra`). NEW helper logic kept private inside my owned command file(s).

## Estimate
Substantial: the similarity-algorithms port (5 algorithms) + 5 reminder strings
(byte-exact) + prefill + step-extraction + dedup scan dominate. **5 points.**
