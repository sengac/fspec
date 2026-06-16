# RPC-199 — `board` AST research (Rust port)

## TS source of truth
- `src/commands/display-board.ts` (128 lines)
- `src/commands/display-board-help.ts` (79 lines)

## Behaviour (verbatim from TS)
`displayBoard({ cwd })`:
1. `checkFoundationExists(cwd, 'fspec board')` — if foundation.json is missing,
   THROWS with the foundation-missing error. (Rust: `io::ensure::check_foundation_exists`
   already exists and produces the verbatim message.)
2. `ensureWorkUnitsFile(cwd)` — auto-creates spec/work-units.json if missing.
   (Rust: `io::ensure::ensure_work_units_file`.)
3. For each `[status, workUnitIds]` in `Object.entries(data.states)`:
   - `columns[status]` = array of `{ id, title, estimate }` looked up from
     `data.workUnits[id]`.
   - `board[status]` = the raw `workUnitIds` array.
   - For each id: if `wu.estimate` truthy: add to `completedPoints` when
     status==='done', else add to `inProgressPoints`.
4. `summary = "${inProgressPoints} points in progress, ${completedPoints} points completed"`.
5. Returns `{ columns, board, summary }`.
6. Any caught error → re-throw `Failed to display board: <message>`.

### BoardResult JSON shape (the parity contract for --format json)
```
{
  "columns": { "<status>": [ { "id", "title", "estimate" }, ... ], ... },
  "board":   { "<status>": [ "<id>", ... ], ... },
  "summary": "N points in progress, M points completed"
}
```
- `title`/`estimate` are OMITTED when the source WorkUnit lacks them
  (TS object literal `{ id: wu.id, title: wu.title, estimate: wu.estimate }`
  yields `title: undefined`/`estimate: undefined`, and `JSON.stringify`
  DROPS undefined-valued keys). So Rust must `skip_serializing_if = "Option::is_none"`
  on `title` and `estimate`.
- Iteration order over `data.states` is the on-disk JSON object key order
  (preserved by IndexMap / serde preserve_order). The canonical order is
  backlog, specifying, testing, implementing, validating, done, blocked.

### Commander registration (`registerBoardCommand`)
- `.command('board')`
- `.option('--format <format>', '...', 'text')` — DEFAULT 'text'.
- `.option('--limit <limit>', '...', '25')` — DEFAULT '25'.
- Action: when `format === 'json'` → `output.log(JSON.stringify(result, null, 2))`.
  Otherwise → renders interactive Ink `BoardDisplay` TUI with `limit`.

## Divergence (interactive TUI) — DECISION NEEDED, flagged to supervisor
The TS text mode renders an **interactive Ink TUI** (`BoardDisplay`) with mouse
support — this cannot be byte-reproduced in a headless Rust CLI test, and the
list-* port precedent is that the Rust standalone binary serves a **plain text /
JSON** surface, not a full TUI re-implementation.

PROPOSED port (mirrors validate.rs envelope + show-coverage rendering):
- Core `board::run` produces the SAME `{columns, board, summary}` data.
  - `format=json` (default for the dispatcher) → 2-space-indented JSON of that
    shape (byte-parity with `JSON.stringify(result, null, 2)`).
  - `format=text` → a deterministic plain-text board rendering (NOT an Ink TUI):
    per-status header + list of `<id>  <title> (<estimate>)` lines, then the
    summary line. This is the headless equivalent; the interactive TUI stays a
    combined-mode concern, out of scope for the subcommand port (same posture as
    list-* commands which render text, not TUI).
- The clap subcommand exposes `--format <format>` and `--limit <limit>` to mirror
  the TS option surface; `--limit` caps items per column in text mode.

**QUESTION for supervisor:** confirm the text-mode rendering format string for
byte-parity, OR decide the CLI default should be `json` (since TS text mode is an
interactive TUI with no stable captured fixture). The `board` help fixture will be
captured verbatim from `node dist/index.js board --help` regardless.

## Foundation-missing path
`board` REQUIRES foundation.json (unlike list-prefixes). Rust core calls
`check_foundation_exists(project_root, "fspec board")` first → on missing returns
`Err(FspecCoreError::FoundationMissing(...))`. The CLI bridge maps Err → exit 1,
stderr (parity with TS throw → output.error + process.exit(1)).

## Shared modules
- `io::ensure::check_foundation_exists` — EXISTS.
- `io::ensure::ensure_work_units_file` — EXISTS (auto-creates).
- `types::work_unit::{WorkUnitsData, WorkUnit, WorkUnitStates}` — EXISTS;
  `WorkUnitsData.states` ordering is preserved. `WorkUnit.estimate` —
  need to confirm a typed `estimate` field exists or read from `extra`.
