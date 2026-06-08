# AST Research — `show-epic` (RPC-302)

This document captures the canonical behaviour of the TypeScript
`show-epic` command (`src/commands/show-epic.ts` + `show-epic-help.ts`),
its sibling `list-epics` (already ported under RPC-243), and the shared
infrastructure I plan to reuse.

## 1. TypeScript surface — file-by-file

### 1.1 `src/commands/show-epic.ts`

Exported entry points:

1. `showEpic(options: { epicId; cwd? }) -> Promise<EpicProgress>` —
   library form returning `{ epic, totalWorkUnits, completedWorkUnits, completionPercentage }`.
2. `showEpicCommand(epicId, options: { format? }) -> Promise<void>` —
   CLI action handler; prints either JSON or text and calls `process.exit(0|1)`.
3. `registerShowEpicCommand(program)` — Commander.js registration:
   - command name `show-epic`
   - description `'Display epic details'`
   - argument `<epicId>` (REQUIRED, positional)
   - option `-f, --format <format>` with default `'text'`
   - action delegates to `showEpicCommand`

### 1.2 Key behaviours observed (line-by-line)

**File reading (lines 42–56):**
- Reads `spec/epics.json` via `readFile`.
- ENOENT (`error.code === 'ENOENT'`) → throws `Error('Epic <epicId> not found')`.
- Any other read error (e.g. EACCES) → re-throws verbatim.
- Successful parse → `epicsData: EpicsData`.

**Epic lookup (lines 58–62):**
- `epicsData.epics[options.epicId]` indexed-access miss → throws
  `Error('Epic <epicId> not found')` (SAME message as ENOENT branch).
- Hit → `epic = epicsData.epics[options.epicId]`.

**Progress aggregation (lines 64–83):**
- Reads `spec/work-units.json` via `readFile`.
- Wrapped in bare `try { ... } catch {}` — ANY failure (ENOENT, parse
  error, EACCES) silently leaves the counts at zero.
- Loops `Object.values(workUnitsData.workUnits)`:
  - `workUnit.epic === options.epicId` → `totalWorkUnits++`.
  - additionally `workUnit.status === 'done'` → `completedWorkUnits++`.

**Percentage calculation (lines 85–88):**
- `totalWorkUnits === 0` → `completionPercentage = 0`.
- otherwise → `Math.round((completed/total) * 100 * 100) / 100`.
- ⚠️ **This is NOT the same as list-epics**: list-epics uses
  `Math.round((completed/total) * 100)` → integer percent
  (e.g. `33`, `67`). show-epic uses 2-decimal rounding
  (e.g. `33.33`, `66.67`). The `* 100 * 100 / 100` idiom multiplies
  by 100 to get a percent, multiplies by 100 again to shift decimal,
  rounds to integer, divides by 100. So 1/3 → 33.33, 2/3 → 66.67,
  1/2 → 50, 4/4 → 100.

**Text output (lines 108–122):**
```
<blank line>
Epic: <epic.id>
<blank line>
Title: <epic.title or 'N/A'>
Description: <epic.description>   ← only if description exists
<blank line>
Progress:
  Total work units: <totalWorkUnits>
  Completed: <completedWorkUnits>
  Completion: <completionPercentage>%
<blank line>
```
- `output.log()` writes each line followed by an implicit newline; the
  blank-`output.log('')` calls emit a literal `\n`.
- `output.log('Title:', result.epic.title || 'N/A')` joins arg list with
  a space → `Title: <title>` or `Title: N/A`.
- `output.log('Description:', result.epic.description)` →
  `Description: <desc>` (only when description truthy).

**JSON output (line 106):**
- `JSON.stringify(result, null, 2)` → 2-space indented.
- Root shape: `{ "epic": { ... }, "totalWorkUnits": N, "completedWorkUnits": N, "completionPercentage": P }`.
- The full Epic object (including any extra fields like `createdAt`) is
  embedded under `epic`.

**Error path (lines 125–133):**
- `output.error('✗', error.message)` → stderr `✗ <message>`.
- `output.error('\nTry: fspec list-epics')` → suggestion.
- `process.exit(1)`.

### 1.3 `src/commands/show-epic-help.ts`

Static help config:
- `name: 'show-epic'`
- `description: 'Display details of an epic including associated work units'`
- `usage: 'fspec show-epic <epicId>'`
- One required argument (`epicId`).
- ⚠️ The help config does NOT declare the `-f, --format` option in
  the `arguments` array, even though Commander.js exposes it. This
  matches the TS pattern for list-epics, where `--format` is similarly
  omitted from the help fixture. We will preserve this asymmetry in the
  Rust port so the help fixture is byte-for-byte identical.
- `examples: [{ command: 'fspec show-epic user-management', ... }]`
- `relatedCommands: ['list-epics', 'create-epic', 'list-work-units']`

## 2. Shared infrastructure already available

- `codelet/fspec-core/src/io/ensure.rs`
  - `read_epics_or_empty(cwd) -> Result<EpicsData>` — RETURNS empty
    on ENOENT. ⚠️ For show-epic this is the WRONG shape: we need
    ENOENT to surface as `Epic <id> not found`, NOT empty.
  - `read_work_units_or_empty(cwd)` — perfect for the bare-`catch {}`
    semantic of the work-units read.
- `codelet/fspec-core/src/types/epic.rs` — `Epic` struct already
  exists with `id`, `title?`, `description?`, `#[serde(flatten)] extra`.
- `codelet/fspec-core/src/types/work_unit.rs` — `WorkUnit` with `epic`
  field; `EpicsData` keyed by `IndexMap<String, Epic>`.
- `codelet/fspec-core/src/error.rs` — `FspecCoreError` includes
  `InvalidArgs { command, reason }` for the "epic not found" path.

## 3. Behaviour deltas vs list-epics

| Aspect | list-epics | show-epic |
|---|---|---|
| Required positional arg | none | `<epicId>` |
| ENOENT epics.json | empty result, success | `Epic <id> not found`, error |
| Empty epics object | success + sentinel | `Epic <id> not found`, error |
| Bad epic id | n/a | `Epic <id> not found`, error |
| Malformed epics.json | escalates `Failed to parse epics.json` | escalates (re-thrown raw read error) |
| Percentage rounding | `Math.round(pct)` → integer | `Math.round(pct*100)/100` → 2dp |
| JSON shape | `{ epics: [array] }` | `{ epic: {...}, totalWorkUnits, completedWorkUnits, completionPercentage }` |
| Text output | `Epics (N)` header per-block | `Epic: <id>` single-block with `Title:` / `Description:` / `Progress:` sections |
| CLI flags | none | `-f, --format <format>` defaulting to `text` |

## 4. New shared helper needed?

For show-epic the ENOENT branch maps to the canonical "epic not found"
error — there is no point auto-creating an empty file. The simplest
approach is to **inline a small read** that mirrors the TS try/catch:

```rust
match std::fs::read_to_string(&path) {
    Ok(raw) => parse → EpicsData,
    Err(e) if e.kind() == ErrorKind::NotFound =>
        return Err(InvalidArgs { reason: format!("Epic {id} not found") }),
    Err(other) => return Err(Io { ... }),
}
```

No new shared helper is strictly required — the read is local to
show-epic. (We could later extract `read_epics_required(cwd)` if other
commands need it.)

## 5. Two-front-doors plan (RPC-003 §7/§11)

- Dispatcher path: `commands::show_epic::run(args_json, &project_root)`.
- CLI path: `codelet/fspec/src/show_epic.rs` clap variant → marshals
  `{ "epicId": <arg>, "format": <flag> }` JSON → SAME `run`.
- Help config: `codelet/fspec-core/src/help/configs/show_epic.rs`
  → registered in `help/configs/mod.rs` (supervisor wires).
- Dispatcher registration: `dispatch.rs` arm (supervisor wires).
- Main clap registration: `codelet/fspec/src/main.rs` Mode variant
  (supervisor wires).

## 6. Files this worker will create/edit

Owned (parallel-safe):
- `codelet/fspec-core/src/commands/show_epic.rs` (rewrite stub).
- `codelet/fspec-core/tests/show_epic.rs` (NEW).
- `codelet/fspec-core/src/help/configs/show_epic.rs` (NEW).
- `codelet/fspec/src/show_epic.rs` (NEW).
- `codelet/fspec/tests/cli_show_epic.rs` (NEW).
- `codelet/fspec/tests/fixtures/help/show-epic.txt` (NEW).
- `spec/features/show-epic-rust-port.feature` (NEW).
- `spec/features/show-epic-cli-subcommand.feature` (NEW).
- `spec/attachments/RPC-302/ast-research-show-epic.md` (THIS file).

Shared (supervisor-only):
- `codelet/fspec-core/src/dispatch.rs`
- `codelet/fspec-core/src/canonical.rs`
- `codelet/fspec-core/src/commands/mod.rs`
- `codelet/fspec-core/src/help/configs/mod.rs`
- `codelet/fspec/src/main.rs`
- `codelet/fspec/tests/cargo_shape.rs`
