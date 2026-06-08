# RPC-308 — `show-work-unit` AST + Behaviour Research

**Phase A — Specifying.** This document captures every behaviour edge in the
TypeScript reference (`src/commands/show-work-unit.ts`, 475 LOC) that the Rust
port MUST preserve. Where appropriate, line references map to the TS source.

## 1. TS Entry Points

### `showWorkUnit(options)` — pure dispatcher-facing function
- **File**: `src/commands/show-work-unit.ts:64-318`
- **Inputs**: `{ workUnitId, output?, verbose?, cwd? }`
- **Reads**:
  - `spec/work-units.json` (REQUIRED — does NOT auto-create; ENOENT escalates)
  - `spec/features/**/*.feature` via `tinyglobby` (best-effort; `try/catch` swallows errors)
- **Throws**: `Error("Work unit '<id>' does not exist")` when the requested
  ID is absent (L76).
- **Returns**: `WorkUnitDetails` — a structured object with the following
  conditionally-present fields:
  - `id`, `title`, `type` (default `"story"`), `status`, `description`,
    `estimate`, `epic`, `parent`, `children`, `blocks`, `blockedBy`,
    `dependsOn`, `relatesTo`, `rules`, `deletedRules` (verbose only),
    `examples`, `questions`, `assumptions`, `architectureNotes`,
    `attachments`, `virtualHooks`, `createdAt`, `updatedAt`,
    `linkedFeatures` (always present, may be empty), `systemReminders`,
    `systemReminder`.

### `showWorkUnitCommand(workUnitId, options)` — CLI wrapper
- **File**: `src/commands/show-work-unit.ts:320-466`
- **CLI registration** (`registerShowWorkUnitCommand`, L468-475):
  ```
  fspec show-work-unit <workUnitId> [-f, --format <format>]
  ```
- `format` defaults to `"text"`. The text rendering applies extensive
  chalk colouring — when piped to non-TTY (NO_COLOR=1) the ANSI escapes
  collapse to identity.

## 2. Behaviour Edges (Rust port MUST preserve)

### 2.1 Soft-Delete Filtering — Per-Category

For every list field with `{deleted: bool, id: u64, text: string, createdAt?,
deletedAt?, selected?: bool}` shape:

| Field              | Filter                              | Format                          |
|--------------------|--------------------------------------|---------------------------------|
| `rules`            | `!r.deleted`                         | `[<id>] <text>` (verbose adds ` (createdAt: <iso>)`) |
| `examples`         | `!e.deleted`                         | `[<id>] <text>`                 |
| `questions`        | `!q.deleted && !q.selected`         | `[<id>] <text>`                 |
| `architectureNotes`| `!n.deleted`                         | `[<id>] <text>`                 |

After filtering: if the resulting array is **empty**, the field is set to
`undefined` (i.e. dropped from JSON output) — see L149-151, L178-180,
L198-200, L210-212.

**Verbose mode** (`verbose: true`) additionally emits `deletedRules` containing
soft-deleted rules with `(deletedAt: <iso>)` suffix when present (L153-168).
Only `rules` get the deleted-companion array — examples/questions/notes do not.

### 2.2 Question Format Validation

If any `questions[]` element is a **string** (not an object), throw:
`Error("Invalid question format. Questions must be QuestionItem objects.")`
(L188-192). The Rust port mirrors via an inline `unwrap_or` chain that
surfaces this as a structured error.

### 2.3 Linked Features Scan

For each `.feature` file under `spec/features/`:
1. Parse with `@cucumber/gherkin` Parser.
2. On parse failure → `continue` (skip silently, L102-106).
3. Call `extractWorkUnitTags(gherkinDocument)` (see `src/utils/work-unit-tags.ts`).
4. Find the matching `WorkUnitTag` whose `id === options.workUnitId`.
5. If found AND `scenarios.length > 0`:
   ```
   linkedFeatures.push({
     file: <relative path>,
     scenarios: [{ name, line, file }]
   })
   ```

`extractWorkUnitTags` rules:
- Tag pattern: `^@([A-Z]{2,6}-\d+)$`.
- Feature-level tags collect ALL scenarios that lack their own
  scenario-level work-unit tag override.
- Scenario-level tags collect only their scenario; if the same ID also
  appears at feature level, level is upgraded to `'scenario'`.
- Scenarios with no name → empty string; `location.line || 0`.

The OUTER `try/catch` around the whole loop (L84-130) swallows any errors —
including "spec/features/ does not exist". So Rust must NOT escalate
`DirectoryNotFound` here; just yield empty `linkedFeatures` and continue.

### 2.4 System Reminder Aggregation

Five potential reminders are concatenated into `systemReminders[]`. When the
array is non-empty, `consolidateReminders(reminders)` produces a single
`<system-reminder>…</system-reminder>` block stored as `systemReminder`.

**Reminder 1 — Missing Estimate (`getMissingEstimateReminder`):**
- Skipped when `process.env.FSPEC_DISABLE_REMINDERS === '1'`.
- Skipped when `estimate !== undefined`.
- Skipped when `status === "backlog"`.
- Otherwise emits a wrapped reminder mentioning the work-unit ID, Fibonacci
  scale, and `fspec update-work-unit-estimate <id> <points>`.

**Reminder 2 — Empty Example Mapping (`getEmptyExampleMappingReminder`):**
- Only checked when `workUnit.status === "specifying"`.
- Skipped when `FSPEC_DISABLE_REMINDERS=1` OR (`hasRules && hasExamples`).
- "Has rules" = `workUnit.rules.some(r => !r.deleted)`.
- "Has examples" = `workUnit.examples.some(e => !e.deleted)`.

**Reminder 3 — Long Duration (`getLongDurationReminder`):**
- Reads `workUnit.stateHistory[length-1].timestamp`.
- Computes `(Date.now() - new Date(ts).getTime()) / (1000*60*60)`.
- Skipped when `< 24` hours.
- Adds per-status advice from a fixed `statusAdvice` map.

**Reminder 4 — Large Estimate (`getLargeEstimateReminder`):**
- Only fires when `type` ∈ {`story`, `bug`} AND `estimate > 13` AND
  `status !== "done"`.
- Distinguishes "has feature file" branch (linkedFeatures.length > 0).

**Reminder 5 — Soft-Delete Count Notice:**
- If `rules.length > 0` AND `deletedCount > 0`, push the **bare string**
  (NOT a wrapped reminder):
  `"${activeCount} active items (${deletedCount} deleted)"`.
- `consolidateReminders` then strips outer tags from any wrapped entries
  AND treats unwrapped strings as plain content, joining with `\n\n` and
  re-wrapping in a single `<system-reminder>` block.

### 2.5 Output-Field Construction (L286-317)

Conditional spreads — fields appear ONLY when truthy/present:
- `description`, `epic`, `parent`, `children`, `blocks`, `blockedBy`,
  `dependsOn`, `relatesTo`, `assumptions`, `attachments`, `virtualHooks`
  → present iff source is truthy.
- `estimate` → present iff `estimate !== undefined` (numeric `0` is kept).
- `rules`, `deletedRules`, `examples`, `questions`, `architectureNotes`
  → present iff their POST-FILTER projection is non-empty.
- `linkedFeatures` → ALWAYS present (may be `[]`).
- `systemReminders`, `systemReminder` → present iff non-empty.
- `type` → always present; defaults to `"story"` when source `wu.type`
  is falsy (matches Rust `WorkUnit::type_str()` semantics).

Field declaration order is preserved by V8 object-insertion order. Rust
must emit JSON with declaration-order fields (use `#[derive(Serialize)]`,
NOT `json!{}` which routes through `BTreeMap`).

## 3. Shared Types To Reuse / Add

### Already available
- `crate::types::work_unit::{WorkUnit, WorkUnitsData}` — provides `extra:
  serde_json::Map` for round-tripping unmodelled fields. We deliberately
  parse `rules` / `examples` / `questions` / `architectureNotes` /
  `assumptions` / `virtualHooks` / `attachments` / `userStory` / etc. INLINE
  from `wu.extra` so the shared type stays minimal (parity with
  `show_deleted.rs` and the "parallel-port-safe" note in RPC-301).
- `crate::io::ensure::ensure_work_units_file` — load-or-init. **NOTE: TS
  show-work-unit does NOT auto-create — it uses bare `readFile` which
  escalates ENOENT.** We will use a **read-only** path: if missing → error
  with substring `Work unit '<id>' does not exist`. Or use the same shared
  helper but with TS-parity semantics: bubble ENOENT as a structured
  `FspecCoreError::Io`. Recommended: read the file directly via
  `std::fs::read_to_string` and on ENOENT return the canonical "Work unit
  does not exist" error (matches TS's ENOENT bubbling up through `readFile`
  → caller sees ENOENT NOT a "work unit does not exist"). **Decision**:
  follow TS exactly — surface raw `Io` for ENOENT (parity with TS
  behaviour). The CLI bridge prints `Error: <io message>`.
- `crate::io::feature_glob::glob_feature_files` — recursive walk, sorted.
  Returns `Err(DirectoryNotFound)` on ENOENT. We MUST swallow that error to
  match TS's bare `try {} catch {}` around the feature-scan block.
- `gherkin` crate (already in `Cargo.toml`) for parsing each feature file.
- `crate::help::{CommandHelpConfig, format_command_help}` — for byte-parity
  `--help` output.

### NOT modelled — read inline from `wu.extra`
- `rules: [{id, text, deleted?, createdAt?, deletedAt?}]`
- `examples: [{id, text, deleted?, createdAt?, deletedAt?}]`
- `questions: [{id, text, deleted?, selected?, createdAt?, deletedAt?}]`
- `architectureNotes: [{id, text, deleted?, createdAt?, deletedAt?}]`
- `assumptions: string[]`
- `attachments: string[]`
- `virtualHooks: [{event, name, command, blocking, gitContext, ...}]`
- `description`, `estimate`, `parent`, `children`, `blocks`, `blockedBy`,
  `dependsOn`, `relatesTo`, `stateHistory`, `userStory`.

## 4. Edge Cases — Specific JSON Shapes

### 4.1 Minimal happy-path work unit
```json
{"id":"AUTH-001","title":"x","status":"backlog","createdAt":"x","updatedAt":"x"}
```
→ Output: `{id, title, type:"story", status, createdAt, updatedAt, linkedFeatures: []}`.

### 4.2 With soft-deleted rule mixed
```json
{"rules":[
  {"id":0,"text":"keep","deleted":false},
  {"id":1,"text":"gone","deleted":true}
]}
```
→ Output: `rules: ["[0] keep"]`. Field `deletedRules` ONLY in verbose mode.

### 4.3 Question filtering
- `{"id":0,"text":"q","deleted":false,"selected":false}` → KEEP.
- `{"id":1,"text":"answered","deleted":false,"selected":true}` → DROP.
- `{"id":2,"text":"gone","deleted":true,"selected":false}` → DROP.

### 4.4 String question (legacy) → error
```json
{"questions":["bare string"]}
```
→ `Error("Invalid question format. Questions must be QuestionItem objects.")`.

### 4.5 specifying status with no rules/examples → emits reminder
- `status: "specifying"`, `rules: []`, `examples: []` → reminder #2 fires.

### 4.6 Large story over 13 points → emits reminder
- `type: "story"`, `estimate: 21`, `status: "implementing"`,
  `linkedFeatures.length === 0` → reminder #4 fires with the
  "CREATE FEATURE FILE FIRST" branch.

### 4.7 Multiple reminders consolidate
- Missing estimate + empty example mapping → both pushed, then
  `consolidateReminders` joins them inside a single `<system-reminder>`.

### 4.8 Disabled reminders
- `FSPEC_DISABLE_REMINDERS=1` → no reminders emitted, output drops both
  `systemReminders` and `systemReminder`.

### 4.9 Linked feature with scenario-level tag
- Given `auth.feature` with `@AUTH-001` on `Feature:` AND `Scenario: Login`
  (no scenario-level tag), the LinkedFeature entry references `Login` at
  the scenario's line.
- Same scenario with its own `@AUTH-002` tag → excluded from AUTH-001's
  linkedFeatures (TS L82 filter: scenarios with their own work-unit-tag
  override are excluded from feature-level inheritance).

### 4.10 Soft-delete count notice
- `rules: [{...,deleted:true}, {...,deleted:false}]` → reminders include
  unwrapped `"1 active items (1 deleted)"` line.

## 5. Text Rendering Layout (TS L331-454)

```
<blank line>
<id>
Type: <type>
Status: <status>
<blank line>
<title>
<description?>                 ← only when present
<blank line>
Epic: <epic>?                  ← only when present
Parent: <parent>?              ← only when present
Children: <a, b, c>?           ← only when children.length > 0
Blocks: <a, b, c>?
Blocked By: <a, b, c>?
Depends On: <a, b, c>?
Related To: <a, b, c>?

Rules:                         ← only when rules.length > 0
  <rule>
  <rule>

Examples:                       ← only when examples.length > 0
  <example>

Questions:                      ← only when questions.length > 0
  <question>

Assumptions:                    ← only when assumptions.length > 0
  1. <text>
  2. <text>

Architecture Notes:             ← only when architectureNotes.length > 0
  <note>

Attachments:                    ← only when attachments.length > 0
  1. <path>

Virtual Hooks:                  ← only when virtualHooks.length > 0
  <event>:                      ← grouped by hook.event
    • <name> (blocking|non-blocking) [git-context]?
      <command>

Linked Features:                ← only when linkedFeatures.length > 0
                                ← blank line
  <featurefile>
    <featurefile>:<line> - <scenario name>

<blank line>
Created: <toLocaleString>
Updated: <toLocaleString>
<blank line>
<systemReminder>?               ← only when present
<blank line>?
```

**Locale-sensitive dates**: TS uses `new Date(iso).toLocaleString()` which
emits the **system locale** representation. Rust must NOT attempt to match
this byte-for-byte — instead the canonical text format must emit raw ISO
strings (parity with the `show_deleted` / `show_epic` precedent of
NOT trying to mimic locale-specific formatting on text output). We will
document this as an INTENTIONAL deviation captured under rule [19].

## 6. JSON Rendering

`output.log(JSON.stringify(result, null, 2))` — pretty 2-space indent.
Rust port uses `serde_json::to_string_pretty(&result)`.

## 7. Wiring Required (supervisor)

After worker impl lands, supervisor will:
1. `codelet/fspec-core/src/help/configs/mod.rs` → add `pub mod show_work_unit;`
2. `codelet/fspec-core/src/dispatch.rs` → move `show-work-unit` from
   `run_stub` (line 375) into `run_ported` arms.
3. `codelet/fspec-core/src/canonical.rs` → add `"show-work-unit"` to
   `PORTED_COMMANDS` (line 199ff).
4. `codelet/fspec/src/main.rs` →
   - Add `mod show_work_unit;`
   - Add `ShowWorkUnit { work_unit_id, format }` Mode variant.
   - Add dispatch arm.
   - Add `"show-work-unit" => format_command_help(&configs::show_work_unit::CONFIG)`
     branch in `intercept_ts_help`.
5. `codelet/fspec-core/src/commands/mod.rs` already declares `pub mod show_work_unit;`
   (stub). No change required there.

## 8. Two-Front-Doors Contract (RPC-003 §7/§11)

- Shell argv → clap → `codelet/fspec/src/show_work_unit.rs` →
  `codelet_fspec_core::commands::show_work_unit::run`.
- LLM tool call JSON → `fspec_core::dispatch::dispatch_command` →
  `codelet_fspec_core::commands::show_work_unit::run`.
- Both call sites pass JSON-encoded args + `project_root: &Path`.
- CLI bridge marshals `workUnitId` (and `format`) into JSON.
- NO business logic in the bridge — `scenario_cli_delegates_to_same_…`
  test scans bridge source for forbidden substrings.

## 9. Decision: ENOENT for `spec/work-units.json`

TS uses bare `readFile` (L70-72) → ENOENT escalates. The Rust port will
**NOT** auto-create. We will:
- `std::fs::read_to_string` returning `Err(io)` for ENOENT.
- Surface as `FspecCoreError::Io { command: "show-work-unit", source }`.
- This bubbles through `dispatch_command` as `success=false, error=...`.

This is a deviation from `show-deleted` (which DOES auto-create) but
matches the TS source-of-truth exactly. Captured under rule [3].
