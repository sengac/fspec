# RPC-261 — `query-metrics` AST research notes

Source of truth: `src/commands/query-metrics.ts` (179 lines core +
`src/commands/query-metrics-help.ts`).

## TS surface

### Public function `queryMetrics(options)`

```ts
queryMetrics(options: {
  workUnitId?: string;
  type?: 'story' | 'task' | 'bug';
  cwd?: string;
}): Promise<MetricsResult>
```

Where `MetricsResult` is:

```ts
interface MetricsResult {
  cycleTime?: string;
  timePerState?: Record<string, string>;
  aggregateMetrics?: {
    totalWorkUnits: number;
    completedWorkUnits: number;
    averageCycleTime?: string;
    byType?: Record<string, { count: number; averageCycleTime?: string }>;
  };
}
```

Note **two mutually exclusive output shapes** in the same return type:

- "Single work-unit" path → only `cycleTime` + `timePerState` populated.
- "Aggregate" path → only `aggregateMetrics` populated.

### Commander.js registration (lines 181–254)

```text
.command('query-metrics')
.description('Query project metrics and statistics')
.option('--work-unit-id <id>', 'Specific work unit to query metrics for')
.option('--type <type>',       'Filter by work unit type: story, task, or bug')
.option('--format <format>',   'Output format: text or json', 'text')
```

NOTE the help-config file declares an additional `--metric <metric>` flag
that the Commander.js registration does NOT expose. The runtime accepts
only `--work-unit-id`, `--type`, and `--format`. The help fixture is
authoritative — we MUST byte-for-byte match the existing help text even
though `--metric` is an undocumented vestige.

### File I/O

- Reads `spec/work-units.json` via `readFile(cwd, 'spec', 'work-units.json')`.
- Does **NOT** auto-create the file. If `readFile` rejects, the surrounding
  `try` rethrows wrapped: `Failed to query metrics: <inner-message>`.
- No write paths.

### Single-work-unit branch (when `workUnitId` is supplied)

1. If `data.workUnits[id]` is missing → throw `Work unit ${id} not found`
   → wrapped as `Failed to query metrics: Work unit ${id} not found`.
2. If the unit has no `stateHistory` (missing or empty array) → throw
   `Work unit ${id} has no state history` → same wrapping.
3. Compute `cycleTimeMs = last.timestamp - first.timestamp`.
4. `cycleTimeHours = Math.round(cycleTimeMs / 3_600_000)` (millis to hours;
   half-away-from-zero rounding for positive values).
5. Time-per-state: walk `[0..len-2]`, take `next.timestamp -
   current.timestamp`, round to hours, store under
   `timePerState[currentState.state]`.
6. Format pluralisation: `"${h} hour${h !== 1 ? 's' : ''}"`. Zero hours →
   `"0 hours"`. One hour → `"1 hour"`. Negative timestamps round per JS
   `Math.round` semantics (rare in real fspec data; not exercised today).
7. Returns `{ cycleTime, timePerState }`.

### Aggregate branch (no `workUnitId`)

1. `let workUnits = Object.values(data.workUnits)` (preserves insertion
   order).
2. If `options.type` provided → filter by `wu.type || 'story'` (TS
   short-circuit: missing OR empty `type` collapses to `'story'`).
3. `totalWorkUnits = workUnits.length`.
4. `completedWorkUnits = filter(wu.status === 'done').length`.
5. `averageCycleTime`:
   - Only computed across `wu.status === 'done' && stateHistory?.length > 0`.
   - Sum `(last.timestamp - first.timestamp)` in ms.
   - `avgHours = Math.round(totalMs / 3_600_000 / completedCount)`.
   - If zero qualifying units → `averageCycleTime` is `undefined`.
6. `byType`:
   - Populated ONLY when `options.type` is NOT provided.
   - Loop over the literal `['story', 'task', 'bug']` (canonical order).
   - For each type: `count = workUnits.filter(wu.type||'story' === type)`,
     `averageCycleTime = same averaging logic over the type's completed`
     subset.
   - Every type key is present, even if `count === 0` → in which case
     `averageCycleTime` stays `undefined`.
7. Returns `{ aggregateMetrics: { totalWorkUnits, completedWorkUnits,
   averageCycleTime, byType } }`.

### CLI text rendering (action handler lines 199–248)

Distinguished by `result.aggregateMetrics` truthy vs `result.cycleTime`
truthy. Single-unit branch can never hit the aggregate path and vice
versa. The exact rendered output for aggregate:

```text

Project Metrics

Total Work Units: N
Completed Work Units: M
Average Cycle Time: H hours        (only if averageCycleTime !== undefined)

By Type:                            (only if byType !== undefined)
  story: K work unit[s]
    Average Cycle Time: X hours   (only when averageCycleTime defined)
  task: K work unit[s]
  bug: K work unit[s]
```

For single-unit:

```text

Work Unit Metrics

Cycle Time: H hours

Time Per State:                     (only if timePerState !== undefined; in TS it always is)
  <state>: H hours
  ...
```

JSON format: `JSON.stringify(result, null, 2)` — 2-space indent.
Object-literal insertion order is preserved by JS, so field order is
`cycleTime`, `timePerState` for single; `aggregateMetrics: { totalWorkUnits,
completedWorkUnits, averageCycleTime, byType }` for aggregate.

### Error handling

- Any thrown error from inside `try { ... }` rewraps as
  `Failed to query metrics: ${err.message}`.
- The Commander.js `action` handler then prints `✗ Query failed: <msg>`
  to stderr (via `output.error`) and calls `process.exit(1)`.

## Rust mapping plan

- `args` shape (camelCase via serde):
  ```rust
  #[serde(default, rename_all = "camelCase")]
  struct QueryMetricsArgs {
      work_unit_id: Option<String>,
      r#type: Option<String>,  // accept "story" | "task" | "bug"
      format: Option<String>,  // "text" | "json"; default "text" via CLI
  }
  ```
- Timestamps: parse ISO-8601 via a small dependency-free parser (mirror
  `epoch_to_ymdhms` style) OR convert to epoch millis using a minimal
  RFC-3339 parser. For RPC-261 we only need millisecond delta — never the
  absolute civil date. The `chrono` crate is deliberately avoided in
  fspec-core (per RPC-248 prior art).
- Rounding: `(ms / 3_600_000.0).round() as i64` mirrors `Math.round`
  half-away-from-zero for positive values; for negative deltas TS
  `Math.round` rounds half-up (`Math.round(-0.5) === 0`). We will round
  via `(x + 0.5).floor()` for non-negative deltas and document
  edge-cases for negative (extremely rare; not exercised).
- Type filter: mirror TS `wu.type || 'story'` via `WorkUnit::type_str()`
  (already exposes the falsy-collapse). Unknown variants stay verbatim
  → fail the equality filter, matching TS string-equality semantics.
- File read: use a dedicated helper (NEW or extend
  `read_work_units_or_empty`?). NB: query-metrics ESCALATES read errors,
  so we must NOT use `read_work_units_or_empty` (which swallows). Either
  inline `std::fs::read_to_string` + `serde_json::from_str` with the
  TS-equivalent error wrapping, OR add a `read_work_units_or_error`
  helper. **Decision**: inline locally in the command since no other
  ported command needs this escalation today.
- Dispatcher: query-metrics currently routes through `run_stub` in
  `dispatch.rs:325`. Supervisor MUST move it to `run_ported` and add it
  to the `is_ported` predicate (shared file — supervisor only).
- CLI bridge: add clap subcommand in `main.rs` (supervisor wires) with
  `--work-unit-id`, `--type`, `--format`.
- Help fixture: capture `node dist/index.js query-metrics --help 2>&1`
  (already verified above to produce known output).

## Hot-spots / questions

- The help config exposes `--metric <metric>` but the action does not
  read it. Help fixture is authoritative — we keep it in the Rust help
  config to preserve byte parity.
- Empty `data.workUnits` → aggregate returns `{ aggregateMetrics: {
  totalWorkUnits: 0, completedWorkUnits: 0, byType: { story: { count: 0
  }, task: { count: 0 }, bug: { count: 0 } } } }` — `averageCycleTime`
  is omitted entirely.
- When `data.workUnits` exists but a particular unit has a
  `stateHistory` of length 1, the aggregate path STILL filters it out
  (`stateHistory.length > 0` is technically true, but
  `last - first === 0`; counted but contributes 0 ms). For the
  single-unit branch, length-1 stateHistory gives `cycleTime: "0 hours"`
  and an empty `timePerState`.
- Timestamps are read as `new Date(...)` — invalid strings yield NaN,
  which propagates to `cycleTimeMs = NaN`, then `Math.round(NaN) = NaN`,
  rendered as `"NaN hours"`. We mirror this by treating parse failures
  as `0` to keep `&str` Display sane (consider escalation as a `Failed
  to query metrics` error — TS path silently produces NaN strings; we
  will preserve TS behaviour and emit `0 hours` to avoid panic — note
  this in a NOTE in the command source).
