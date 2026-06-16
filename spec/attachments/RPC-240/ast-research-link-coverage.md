# RPC-240 — link-coverage — AST / Port Research (Worker 3)

## TS source
- `src/commands/link-coverage.ts` — `linkCoverage(featureName, options)` + command + register.
- Helpers under `src/commands/link-coverage/`:
  - `validator.ts` — `validateFlagCombinations`, `validateFiles` (file existence + skip-validation warnings).
  - `mapping-ops.ts` — `addTestMapping`, `addImplMapping`, `addBothMappings`, `parseImplLines` (impl lines string → number[]), `getRemovalHint` (chalk gray).
  - `stats-updater.ts` — `updateStats` (same shape as unlink-coverage local update_stats; totalLinesCovered = test ranges + impl line array lengths).
  - `step-validator.ts` — `validateStepConsistency` (step @comment matching, MANDATORY for story/bug).
  - `utils.ts` — `wrapSystemReminder`, `detectWorkUnitType` (reads @WORK-UNIT-ID tag + work-units.json), `getScenariosFromFeatureFile` (regex `^\s*Scenario(?:\s+Outline)?:\s*(.+)$`).
- `src/utils/step-validation.ts` — `extractStepComments`, `validateSteps`, `matchStep`, `formatValidationError`, hybrid similarity.
- `src/utils/similarity-algorithms.ts` — jaroWinkler, tokenSet, trigram, jaccard, gherkinStructural, hybridSimilarity (adaptive thresholds by text length).

## Operation modes (after validation)
1. test-only: `--test-file --test-lines` (no impl-file) → addTestMapping → `✓ Linked test mapping: <tf>:<lines>` (or `second test mapping` if dup file).
2. impl-only: `--test-file --impl-file --impl-lines` (no test-lines) → addImplMapping (finds existing test mapping; errors `Test mapping not found: <tf>` if absent; updates if impl file present else adds) → `✓ Added/Updated implementation mapping: <if>:<lines>`.
3. both: `--test-file --test-lines --impl-file --impl-lines` → addBothMappings → `✓ Linked test mapping with implementation: <tf>:<tl> → <if>:<il>`.
Else → `Invalid flag combination` error.

Result message has `getRemovalHint` appended (chalk.gray unlink hint).

## Flag validation (validator.ts, fires before file load)
- impl-file without test-file → `--test-file is required when adding implementation mappings\nImplementation mappings attach to specific test mappings`.
- test-file without impl-file AND without test-lines → `--test-lines is required when linking test file\nExample: ...`.
- impl-file without impl-lines → `--impl-lines is required when linking implementation file\nExample: ...`.

## File validation (validator.ts)
- Unless `--skip-validation`: test-file/impl-file must exist (join cwd) else `File not found: <path>\nSuggestion: ...`.
- With `--skip-validation`: missing files just push warnings `⚠️  File not found: <path> (validation skipped)`.

## Coverage load
- path = `spec/features/<name>.feature.coverage` (tolerate trailing `.feature`).
- Missing/unreadable: if feature file has scenarios → wrapped system-reminder + `Coverage file not found: <name>.feature.coverage\nSuggestion: Run 'fspec generate-coverage' ...`; else → `Coverage file not found: <name>.feature.coverage\nSuggestion: Run 'fspec create-feature' ...`.
- Scenario not in coverage: if exists in feature file → system-reminder (out of sync, run generate-coverage) + `Scenario not found: "<s>"\nAvailable scenarios:\n  - ...`; else → bare `Scenario not found ...`.

## Step validation (MANDATORY for story/bug; skippable only for task)
- If `--skip-step-validation` AND testFile: detect work unit type. story/bug → throw enforcement system-reminder + tail. task → push warning, skip.
- Else if testFile: `validateStepConsistency` — parse feature scenario steps (`<keyword.trim()> <text>`), extract `@step`/plain comments from test file, hybrid-similarity match each feature step; if any missing → `formatValidationError(...) + '\n\nStep validation failed'`. Feature file ENOENT → skip silently.

## Write-back
- `fileManager.transaction(coverageFile, fd => Object.assign(fd, coverage))` → atomic 2-space JSON. Rust: `write_json_atomic`. Preserve unknown fields (extra-flatten).

## Rust port plan
- Rewrite stub `codelet/fspec-core/src/commands/link_coverage.rs`: `pub async fn run(args_json: &str, project_root: &Path)`.
  - SHARED-FILE REQUEST (supervisor): dispatch arm `run(args_json)` → `run(args_json, project_root)`.
- Reuse `types::coverage` — REUSE only. Local `update_stats` mirrors stats-updater (NOT shared calculate_stats; totalLinesCovered = test ranges + impl array len). Identical to unlink_coverage's local update_stats — but I must NOT touch unlink_coverage.rs; I'll duplicate the helper inside link_coverage.rs (small, parity-safe).
- `parseImplLines`: comma/range string → Vec<i64>; store as ImplLines::Array.
- detect_work_unit_type: read feature file @WORK-UNIT-ID tag (regex `@([A-Z]+-\d+)`), look up work-units.json `workUnits[id].type` (default "story"). Reuse types::work_unit + io::ensure or read raw. DECISION: read work-units.json via existing io helper (read-only); fall back to "story" on any error.
- Step validation similarity: PORT the hybrid similarity + step extraction into a LOCAL module inside link_coverage.rs (no shared module per ownership rules). This is the heaviest part. Algorithms: jaroWinkler, tokenSet, trigram, jaccard, gherkinStructural, weighted hybrid + adaptive thresholds (0.85/<10, 0.80/<20, 0.75/<40, 0.70/40+). @step regex: `@step\s+(Given|When|Then|And|But)\s+(.+?)(?:\s*\*\/.*)?$` and plain `^//\s+(Given|When|Then|And|But)\s+(.+)$`.
- Envelope: `{ success, message, warnings? }` returned; CLI bridge prints message + yellow warnings (parity). Errors → InvalidArgs{reason} so bridge prints `Error: <reason>`.

## SHARED-FILE REQUESTS (for supervisor)
1. dispatch.rs: change `generate-coverage` and `link-coverage` arms to pass `project_root`, and move both into `run_ported` (add to PORTED_COMMANDS list in canonical.rs + run_ported match arms). [Both my commands.]
2. commands/mod.rs, help/configs/mod.rs, main.rs (Mode variants + intercept arms + forward!) — supervisor wiring in PHASE C.
3. types::coverage.rs: NO extension needed — REUSE as-is.

## Estimation note
link-coverage is the heaviest port of the two due to the hybrid step-similarity algorithm port (~5 algorithms) + multi-mode mutation + system-reminder error parity. Likely 8 points. generate-coverage ~5.
