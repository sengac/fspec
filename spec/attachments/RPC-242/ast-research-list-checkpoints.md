# AST Research — list-checkpoints (RPC-242)

## TypeScript source of truth

### `src/commands/list-checkpoints.ts`

- Public surface: `listCheckpoints({ workUnitId, cwd })` and `registerListCheckpointsCommand(program)`.
- Commander.js registration:
  - `.command('list-checkpoints')`
  - `.description('List all checkpoints for a work unit')`
  - `.argument('<work-unit-id>', 'Work unit ID (e.g., AUTH-001)')`
  - NO `.option(...)` calls — **flag-less surface**, mirroring `list-prefixes`.
- Behavior:
  1. Delegates to `listCheckpoints(workUnitId, cwd)` from `src/utils/git-checkpoint.ts`.
  2. If empty → `output.log("No checkpoints found for ${workUnitId}")`; returns `{ checkpoints: [] }`.
  3. Otherwise:
     - Prints blank line + `Checkpoints for ${workUnitId}:` + blank line.
     - For each checkpoint:
       - Icon: `🤖` if automatic (contains `-auto-`), else `📌`.
       - Label: `(automatic)` (gray) or `(manual)` (blue).
       - Line 1: `${icon}  ${chalk.bold(cp.name)} ${typeLabel}` (note the **two spaces** after icon).
       - Line 2: `   Created: ${cp.timestamp}` (three spaces of indent).
       - Blank line.
  4. On error: stderr `Error: <message>`, exit 1.

### `src/utils/git-checkpoint.ts::listCheckpoints`

1. Reads index file `.git/fspec-checkpoints-index/{workUnitId}.json` (missing/corrupt → empty checkpoints array).
2. Calls NAPI `listGhostCheckpoints(cwd, workUnitId)` → `Vec<String>` of checkpoint names (refs under `refs/fspec-checkpoints/{workUnitId}/`).
3. For each checkpoint name:
   - Looks up timestamp in index by name match; defaults to `new Date().toISOString()` if not found.
   - `isAutomatic = name.includes('-auto-')`.
4. Sorts by `new Date(timestamp).getTime()` **descending** (newest first).
5. Returns `Checkpoint[]` with `{ name, workUnitId, timestamp, stashRef, isAutomatic, message }`.

### `src/utils/checkpoint-index.ts::isAutomaticCheckpoint`

- `name.includes(AUTO_CHECKPOINT_PATTERN)` where `AUTO_CHECKPOINT_PATTERN = '-auto-'`.

### `src/commands/list-checkpoints-help.ts`

- Has `arguments: [{ name: 'workUnitId', required: true, description: ... }]`.
- `options: []`.
- 2 examples (populated + empty case).
- 2 prerequisites, 5 typical workflow steps, 2 common errors, 4 related commands, 6 notes.

## Rust port mapping

### Reusable infrastructure

- **`codelet_git::ghost_commit::list_ghost_checkpoints(dir, workUnitId) -> Result<Vec<String>>`** — already exists at `codelet/git/src/ghost_commit.rs:552`. Iterates refs under `refs/fspec-checkpoints/{workUnitId}/`. **Gracefully returns Err only if `open_repo` fails (not a git repo) — TS swallows this and returns empty list.**
- **`AUTO_CHECKPOINT_PATTERN = "-auto-"`** — already exported from `codelet_git::ghost_commit`.
- Need a new helper `read_checkpoint_index_or_empty(cwd, workUnitId)` that reads `.git/fspec-checkpoints-index/{workUnitId}.json` and returns empty on ENOENT or parse error (parity with TS `try { JSON.parse } catch {}`).

### New dependency

**`codelet/fspec-core/Cargo.toml` MUST gain `codelet-git.workspace = true`** — needed for `list_ghost_checkpoints` and `AUTO_CHECKPOINT_PATTERN`.

### Error handling

- If `open_repo` (not a git repo) → match TS: silently return empty list (NOT an error). TS's `listGhostCheckpoints` NAPI throws but the surrounding command-level error path is `output.error('Error:', ...)` + exit 1. **However**, looking at TS more carefully: `listCheckpoints` does not catch NAPI errors itself — it lets them propagate to the outer try/catch in `listCheckpointsCommand`. So if NAPI throws, exit 1.
- For Rust port: prefer "not a git repo" → empty list (safer for tests), and only escalate genuine gix errors. This matches `count_checkpoints` semantics already shipped in codelet-git.

### File layout (worker-only)

1. `codelet/fspec-core/src/commands/list_checkpoints.rs` (rewrite stub)
2. `codelet/fspec-core/src/help/configs/list_checkpoints.rs` (new help config)
3. `codelet/fspec-core/tests/list_checkpoints.rs` (new dispatcher test)
4. `codelet/fspec/src/list_checkpoints.rs` (new CLI bridge)
5. `codelet/fspec/tests/cli_list_checkpoints.rs` (new CLI test)
6. `codelet/fspec/tests/fixtures/help/list-checkpoints.txt` (new help fixture)

### Shared-file changes (SUPERVISOR)

1. **`codelet/fspec-core/Cargo.toml`** — add `codelet-git.workspace = true` to `[dependencies]`.
2. **`codelet/fspec-core/src/help/configs/mod.rs`** — add `pub mod list_checkpoints;`.
3. **`codelet/fspec-core/src/dispatch.rs`** — register `list-checkpoints` route (replace NotYetPorted with `list_checkpoints::run(...)`).
4. **`codelet/fspec-core/src/canonical.rs`** — already contains `list-checkpoints` (it's stubbed, not absent).
5. **`codelet/fspec/src/main.rs`** — add `Mode::ListCheckpoints { work_unit_id: String }` clap variant, add the action arm, add the help-handling block (`if --help`), and dispatch to `list_checkpoints::run`.
6. **`codelet/fspec-core/src/io/ensure.rs`** — (PROPOSED) add `read_checkpoint_index_or_empty(cwd, work_unit_id)` helper. Returns `Vec<{name, timestamp}>`. **Could alternatively live inside `commands/list_checkpoints.rs` as a private fn — preferred to minimise shared-file churn.**

### Worker plan (private helper inside command module)

To minimize shared-file change requests, keep the index-reading helper *inside* `commands/list_checkpoints.rs` as a private function. The only required supervisor changes are:
- `Cargo.toml` dependency add
- `help/configs/mod.rs` module add
- `dispatch.rs` route
- `main.rs` clap variant + dispatch

## Acceptance criteria distillation

### Dispatcher path (`*-rust-port.feature`)

1. `workUnitId` argument is REQUIRED — missing arg → `InvalidArgs` failure.
2. No git repo at project_root → success with empty checkpoints list (graceful, matches `count_checkpoints` semantics).
3. Git repo exists but NO checkpoints registered for workUnitId → success with empty list + sentinel `"No checkpoints found for AUTH-001"`.
4. One manual checkpoint registered + index file has timestamp → text output shows `📌  baseline (manual)` line + `   Created: <timestamp>`.
5. One automatic checkpoint registered → text output shows `🤖  AUTH-001-auto-testing (automatic)` line.
6. Mixed auto + manual checkpoints with timestamps in index → sorted by timestamp DESC (newest first).
7. Checkpoint exists in git refs but NOT in index → timestamp defaults to **some non-empty ISO-8601 string** (current time fallback). Tests should NOT depend on exact wall-clock; assert non-empty format.
8. `format: "json"` → returns JSON `{ "workUnitId": "AUTH-001", "checkpoints": [{ "name", "timestamp", "displayIcon", "isAutomatic" }] }`.
9. Empty workUnitId string → still treated as valid arg, returns empty list (TS does not validate).
10. Malformed index file (`{ not json`) → treated as empty index, timestamps fall back, command still succeeds.
11. Insertion order from git refs is NOT preserved — sort by timestamp DESC.

### CLI subcommand path (`*-cli-subcommand.feature`)

12. `fspec list-checkpoints --help` → exits 0, byte-for-byte matches fixture, mentions `<workUnitId>` argument, NO `--format/--workspace/--prefix` flags.
13. `fspec list-checkpoints` (missing arg) → clap error → exit 2 (clap default for missing required arg).
14. `fspec list-checkpoints AUTH-001` against empty dir → exits 0, stdout `No checkpoints found for AUTH-001`.
15. `fspec list-checkpoints AUTH-001` against repo with checkpoints → exits 0, stdout contains `Checkpoints for AUTH-001:`, the `📌`/`🤖` icons, and the `(manual)`/`(automatic)` labels.
16. Default combined TUI mode is preserved (list-checkpoints registered alongside list-prefixes, list-work-units, etc).
17. CLI delegates to `fspec_core::commands::list_checkpoints::run` — no duplicated rendering logic in bridge.

## Estimate

5 points. Reasons:
- New `codelet-git` dependency on `fspec-core` — small but cross-crate change requiring supervisor coordination.
- Real git-repo fixture setup in tests (init repo + write refs + write index file) — moderately involved.
- Logic itself is straightforward (list → lookup timestamp → classify → sort → render).
- Two front-doors (dispatcher + CLI) like list-prefixes.
