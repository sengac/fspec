# AST Research — `record-iteration` (RPC-264)

TS source of truth: `src/commands/record-iteration.ts` (+ `record-iteration-help.ts`)
Rust stub: `codelet/fspec-core/src/commands/record_iteration.rs` (NotYetPorted, RPC-264)

## 1. TS surface inventory

### Exported function `recordIteration(options)` — lines 21-56

```ts
recordIteration(options: { workUnitId: string; cwd?: string })
  : Promise<{ success: boolean; iterations?: number }>
```

Behaviour:
1. `cwd = options.cwd || process.cwd()`.
2. `workUnitsFile = join(cwd, 'spec', 'work-units.json')`.
3. `readFile(workUnitsFile, 'utf-8')` then `JSON.parse` → `WorkUnitsData`.
   - **No `ensureWorkUnitsFile`** — direct `readFile`. ENOENT / parse errors
     are caught and re-thrown wrapped (see step 6).
4. If `!data.workUnits[options.workUnitId]` → `throw new Error(\`Work unit ${id} not found\`)`.
5. Mutate the found work unit:
   - `workUnit.iterations = (workUnit.iterations || 0) + 1;` (post-init 0 → +1).
   - `workUnit.updatedAt = new Date().toISOString();`
6. `writeFile(workUnitsFile, JSON.stringify(data, null, 2))` — **2-space indent, plain `writeFile` (no atomic/lock)**.
7. Return `{ success: true, iterations: workUnit.iterations }`.
8. catch: `throw new Error(\`Failed to record iteration: ${error.message}\`)`.

### CLI registration `registerRecordIterationCommand(program)` — lines 58-77

```ts
program
  .command('record-iteration')
  .description('Record an iteration or sprint')
  .argument('<name>', 'Iteration name')
  .option('--start <date>', 'Start date')
  .option('--end <date>', 'End date')
  .action(async (name, options) => {
    await recordIteration({ name, start: options.start, end: options.end });
    output.log(`✓ Iteration recorded successfully`);
  });
// catch → output.error('✗ Failed to record iteration:', error.message); process.exit(1)
```

## 2. CRITICAL TS BUG — CLI / function param mismatch ("Framing A")

The CLI action calls `recordIteration({ name, start, end })`, but the function
ONLY reads `options.workUnitId`. So:
- `options.workUnitId` is `undefined` at runtime.
- `data.workUnits[undefined]` is undefined → step 4 throws
  `Work unit undefined not found`.
- The catch wraps: `Failed to record iteration: Work unit undefined not found`.
- → `output.error('✗ Failed to record iteration:', ...)` + `process.exit(1)`.

**So `node dist/index.js record-iteration "Sprint 1"` ALWAYS fails with exit 1**
regardless of work-unit state. The `--start`/`--end` flags and the `<name>`
argument are silently discarded by the function. The success log
`✓ Iteration recorded successfully` is unreachable via the shell CLI.

### Framing A decision (per command-port.md §10)
- The **dispatcher / function contract** is the canon: `recordIteration({workUnitId, cwd})`
  increments `iterations` and bumps `updatedAt`. The Rust `commands::record_iteration::run`
  implements THIS (the useful function).
- The **CLI shell** mirrors the broken TS behaviour: the clap subcommand exposes
  `<name>` + `--start` + `--end` (Commander surface), marshals NO `workUnitId`,
  so the core fn throws `Work unit undefined not found` → exit 1.
- Help-doc (`record-iteration-help.ts`) is canon for `--help` output ONLY.

## 3. Help config (`record-iteration-help.ts`)

```
name: 'record-iteration'
description: 'Record an iteration or sprint with metadata'
usage: 'fspec record-iteration <name> [options]'
arguments: [ name (required): 'Iteration name (e.g., "Sprint 1", "Week 42")' ]
options:
  --start <date> : Start date (ISO format)
  --end <date>   : End date (ISO format)
examples:
  fspec record-iteration "Sprint 1" --start 2025-10-01 --end 2025-10-15
    desc: Record iteration
    output: ✓ Recorded iteration "Sprint 1"
relatedCommands: ['query-metrics', 'generate-summary-report']
```
(Note: example output differs from runtime — help-doc is canon for `--help`.)

## 4. Dispatcher contract (function canon) — Rust core fn

`pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>`

Args (camelCase):
```rust
struct Args { work_unit_id: Option<String>, /* cwd ignored — project_root passed */ }
```

Logic:
1. Read `spec/work-units.json` DIRECTLY (no auto-create). ENOENT → error wrapped
   `Failed to record iteration: <io msg>`. Parse error → wrapped similarly.
   (Mirror `query_work_units.rs` direct-read + `format_io_error` pattern.)
2. If `workUnitId` missing in map (incl. when None → key "undefined" semantics):
   wrap `Failed to record iteration: Work unit <id> not found`.
   - For the CLI path where workUnitId is absent, TS literally produces the
     string `Work unit undefined not found`. Rust core: when `work_unit_id` is
     None, treat as the literal id `"undefined"` to reproduce the exact message.
     (Alternatively the bridge passes `workUnitId: "undefined"`; decide in PHASE B.)
3. `iterations = (iterations || 0) + 1` — read from `extra` (Number), write back.
4. `updatedAt = iso8601_now()`.
5. Write back with **2-space pretty JSON** preserving field order
   (`write_json_atomic` already pretty-prints w/ preserve_order — confirm 2-space).
6. Return JSON `{ "success": true, "iterations": <n> }`.

## 5. Shared infra reused (NO new shared files needed)
- `crate::types::work_unit::{WorkUnitsData, WorkUnit}` — `iterations` lives in `extra`.
- `crate::io::locked_file::write_json_atomic` — atomic 2-space write w/ preserve_order.
- `crate::io::time::iso8601_now` — `updatedAt`.
- Direct `std::fs::read_to_string` for the no-auto-create read (like query_work_units).

## 6. Files to produce (6 artifacts)
1. `codelet/fspec-core/src/commands/record_iteration.rs` (rewrite stub)
2. `codelet/fspec/src/record_iteration.rs` (CLI bridge)
3. `codelet/fspec-core/src/help/configs/record_iteration.rs` (help config)
4. `codelet/fspec/tests/fixtures/help/record-iteration.txt` (help fixture)
5. `codelet/fspec-core/tests/record_iteration.rs` (dispatcher test)
6. `codelet/fspec/tests/cli_record_iteration.rs` (CLI test)

Shared-file changes (SUPERVISOR): canonical.rs PORTED_COMMANDS, dispatch.rs arm,
commands/mod.rs, help/configs/mod.rs, main.rs Mode + intercept + bridge mod.

## 7. Estimate: 2 (simple) — ~80 LOC TS, single work-unit mutation, one new shared concern (iterations in extra), reuses all infra.
