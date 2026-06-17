# AST research — `auto-advance` (RPC-198)

## TS source: `src/commands/auto-advance.ts`

### Exported core function `autoAdvance(options)`
Signature: `autoAdvance({ workUnitId, from, event, cwd? }): Promise<{ success: boolean; newState?: string }>`

Behaviour:
1. Resolves `cwd` (default `process.cwd()`), reads `spec/work-units.json` via `readFile` + `JSON.parse`.
2. If `data.workUnits[workUnitId]` missing → throw `Work unit ${workUnitId} not found`.
3. Finds transition in static `STATE_TRANSITIONS` table matching `from` + `event`:
   - `{ from: 'testing',    event: 'tests-pass',      to: 'implementing' }`
   - `{ from: 'validating', event: 'validation-pass', to: 'done', recordCompletion: true }`
   - No match → throw `No transition defined for ${from} + ${event}`.
4. If `workUnit.status !== from` → throw `Work unit is in ${workUnit.status} state, expected ${from}`.
5. Removes id from `data.states[from]` array (filter), pushes id into `data.states[to]` (create array if absent).
6. Sets `workUnit.status = transition.to`, `workUnit.updatedAt = new Date().toISOString()`.
7. If `recordCompletion` → also set `workUnit.completedAt = new Date().toISOString()`.
8. Atomic write via `fileManager.transaction(workUnitsFile, ...)` (= `write_json_atomic` in Rust).
9. Returns `{ success: true, newState: transition.to }`.
10. ALL errors wrapped: `throw new Error('Failed to auto-advance: ' + error.message)`.

### Commander registration `registerAutoAdvanceCommand` — FRAMING A (BROKEN SHELL)
```ts
program.command('auto-advance')
  .description('Automatically advance work units through workflow states')
  .option('--dry-run', 'Show what would be advanced without making changes')
  .action(async (options: { dryRun?: boolean }) => {
    const result = await autoAdvance({ dryRun: options.dryRun });   // <-- passes ONLY dryRun
    output.log(`✓ Advanced ${result.advanced} work units`);         // <-- result.advanced does not exist
    if (result.details && result.details.length > 0) { ... }        // <-- result.details does not exist
  });
```

**The CLI shell is BROKEN (Framing A).** The `.action()` calls `autoAdvance({ dryRun })` — it does NOT
pass `workUnitId`, `from`, or `event`. The function therefore reads `data.workUnits[undefined]`, which is
always missing → throws `Work unit undefined not found`, caught & wrapped as
`Failed to auto-advance: Work unit undefined not found`, then `output.error('✗ Failed to auto-advance:', msg)`
and `process.exit(1)`. The happy-path log lines (`result.advanced`, `result.details`) are unreachable and
reference fields the function never returns.

This mirrors `record-iteration` (RPC-264) Framing A: the Rust CLI bridge must reproduce the broken shell
(send NO workUnitId so the core deterministically surfaces `Work unit undefined not found` and exits 1).

The `-help.ts` (`auto-advance-help.ts`) describes an ASPIRATIONAL command ("auto-advance eligible work units",
`--dry-run`) that does NOT match the actual implementation — there is no dry-run logic in `autoAdvance` and
no eligibility scan. The help fixture is captured verbatim from `node dist/index.js auto-advance --help`
(rich `formatCommandHelp`), independent of the broken `.action()`.

## Rust port mapping
- Core `run(args_json, project_root)` mirrors `autoAdvance(options)`: args `{ workUnitId, from, event }`
  (all `Option<String>`, default → `undefined` sentinel for Framing A parity on missing id).
- Reuse `WorkUnitsData` / `WorkUnitStatus`, `write_json_atomic`, `iso8601_now`. Round-trip raw object tree
  (like `update_work_unit.rs`) to preserve key order + unmodelled fields; mutate `states` arrays + the unit.
- Errors wrapped with `Failed to auto-advance:` prefix.
- CLI bridge = Framing A: marshal `{}` (no workUnitId) → core surfaces `Work unit undefined not found`,
  bridge prints `✗ Failed to auto-advance: ...` to stderr, exit 1. `--dry-run` flag accepted but ignored.
