# AST Research — `update-work-unit-estimate` (RPC-318)

## TS source of truth
- `src/commands/update-work-unit-estimate.ts` (160 LOC)
- `src/commands/update-work-unit-estimate-help.ts`
- `src/utils/prefill-detection.ts` (checkWorkUnitFeatureForPrefill + detectPrefill)

## Public surface
Commander.js registration (`registerUpdateWorkUnitEstimateCommand`):
- positional `<workUnitId>` (required)
- positional `<estimate>` (required) — parsed with `parseInt(estimate, 10)`

## FIBONACCI_NUMBERS = [1, 2, 3, 5, 8, 13, 21]

## Core function `updateWorkUnitEstimate(options)` behaviours

1. **Fibonacci validation** — `if (!FIBONACCI_NUMBERS.includes(estimate))` throw
   `"Invalid estimate: <estimate>. Must be one of: 1,2,3,5,8,13,21"`. (33-37)
2. **Read work-units** — `fileManager.readJSON(workUnitsFile, {workUnits:{},states:{}})`
   — read-or-default, does NOT auto-create file. (40-43)
3. **Existence check** — `if (!data.workUnits[id]) throw "Work unit <id> not found"`. (46-48)
4. **ACDD gate (story/bug/undefined type)** — if `wu.type === 'story' || wu.type === 'bug' || !wu.type`:
   call `checkWorkUnitFeatureForPrefill(id, cwd)`:
     - returns `null` when NO feature file has `@<id>` tag → throw the
       "ACDD VIOLATION: Cannot estimate ... without completed feature file" system-reminder block. (65-88)
     - returns result with `hasPrefill: true` → throw the "ACDD VIOLATION: Cannot
       estimate work unit with incomplete feature file" system-reminder block listing
       up to 3 placeholder matches. (91-119)
   - Tasks (`wu.type === 'task'`) are EXEMPT — skip the gate entirely.
5. **Update** — `wu.estimate = estimate; wu.updatedAt = new Date().toISOString()`. (123-124)
6. **Atomic write** — `fileManager.transaction(workUnitsFile, ...)`. (126-129)
7. **Result** — `{ success: true }`. (131)
8. **Outer catch** — ALL thrown errors re-wrapped: `throw new Error("Failed to update work unit estimate: " + error.message)`. (132-137)

## checkWorkUnitFeatureForPrefill (prefill-detection.ts)
- featuresDir = `<cwd>/spec/features`; if missing → return null.
- readdir; for each `*.feature`, read content; tag regex `(^|\s)@<id>(?:\s|$)` multiline.
- first file whose content matches the tag → `detectPrefill(content)`.
- if no file matches → return null.

### detectPrefill(content)
PREFILL_PATTERNS (regex / name / command):
- `/\[role\]/gi` → `[role]`
- `/\[action\]/gi` → `[action]`
- `/\[benefit\]/gi` → `[benefit]`
- `/\[precondition\]/gi` → `[precondition]`
- `/\[expected outcome\]/gi` → `[expected outcome]`
- `/\[scenario name\]/gi` → `[scenario name]`
- `/TODO:/gi` → `TODO:`
- `/^@.*@component(?!\w)/gm` → `@component` (multiline)
- `/^@.*@feature-group(?!\w)/gm` → `@feature-group` (multiline)
Returns `{ hasPrefill, matches[], systemReminder? }`. Each match: pattern, line, context, suggestion.

## CLI output (registerUpdateWorkUnitEstimateCommand)
- success: `chalk.green("✓ Work unit <id> estimate set to <estimate>")`
- error: `output.error("✗ Failed to update estimate:", error.message)` + `process.exit(1)`

## Rust mapping plan
- Core: `commands/update_work_unit_estimate.rs` — `pub async fn run(args_json, project_root)`.
- Args struct: `workUnitId: String, estimate: i64` (camelCase). CLI parses estimate string → number in bridge OR pass as number. TS uses parseInt; bridge marshals as JSON number.
- Read work-units WITHOUT auto-create: use `read_work_units_or_empty`? NO — TS `readJSON` with default returns empty store on ENOENT but ESCALATES parse error? Actually fileManager.readJSON swallows ENOENT→default. `read_work_units_or_empty` swallows BOTH ENOENT and parse error. Closest match: TS readJSON returns default on missing; on existing+parse-fail it throws. Need to confirm fileManager.readJSON semantics. For parity pick the helper whose ENOENT→empty AND parse-error behaviour matches. Likely `read_work_units_or_empty` (ENOENT→empty) is acceptable; document the parse-error nuance.
- Fibonacci constant `[1,2,3,5,8,13,21]`.
- Prefill detection: port the minimal needed bits into an ISOLATED module within
  the command file (or a new helper). The 9 prefill patterns + tag-match regex.
  ASK SUPERVISOR before adding to io/ensure.rs — prefer a private helper module
  inside commands/update_work_unit_estimate.rs to stay file-isolated.
- Need feature-file glob: `io::feature_glob::glob_feature_files()` exists — but TS
  uses flat readdir of spec/features (NOT recursive). Mirror TS: flat readdir of
  spec/features/*.feature only. Implement a small flat reader in the command module.
- `estimate` & `updatedAt` set on WorkUnit; `estimate` lives in `extra` (not typed),
  `updated_at` is typed.
- All errors wrapped with `"Failed to update work unit estimate: "` prefix.
- The system-reminder blocks must be byte-exact ports of the TS template literals
  (trimmed). Two big multi-line strings.
- Result JSON `{ "success": true }`.

## Regex porting note (no regex crate in runtime path)
- Tag match `(^|\s)@<id>(?:\s|$)` → hand-roll: scan content lines; for the tag to
  match, `@<id>` must be preceded by start-or-whitespace and followed by
  whitespace-or-end. Implement with a manual scan.
- Prefill brackets are literal substring matches (case-insensitive) for the bracket
  patterns; `TODO:` case-insensitive; `@component`/`@feature-group` multiline `^@...`
  line-prefixed. Hand-roll line scanning.

## Shared-file asks for supervisor
- Reuse `read_work_units_or_empty` (io/ensure.rs) — no change. Confirm parse-error parity acceptable.
- PORTED_COMMANDS / dispatch / mod.rs / main.rs / help configs mod (ALL supervisor-owned).
