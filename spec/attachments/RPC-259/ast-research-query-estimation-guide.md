# RPC-259 — AST Research: `query-estimation-guide`

## Source of truth (TypeScript)

- Implementation: `src/commands/query-estimation-guide.ts`
- Help config: `src/commands/query-estimation-guide-help.ts`

## TS behavioural inventory

### 1. Inputs

- Constructor option: `cwd?: string` (defaults to `process.cwd()`).
- Reads `spec/work-units.json` directly via `readFile(join(cwd, 'spec', 'work-units.json'), 'utf-8')` — does NOT use `ensureWorkUnitsFile`. ENOENT propagates as error.
- CLI argument: `<workUnitId>` (positional, REQUIRED in Commander.js sense).
- CLI flag: `--format <format>` (default `text`).

### 2. Surprising behaviours

- The CLI registration declares positional `<workUnitId>` but the `queryEstimationGuide` function does NOT consume it — the function only computes global patterns over all done work units. The argument is gathered by Commander.js but never actually used (TS source bug we replicate).
- The function emits patterns derived ONLY from completed (`status === 'done'`) work units that have BOTH `estimate` and `iterations` fields populated.
- When `format !== 'json'`, the CLI prints **NOTHING** to stdout (only the JSON branch logs). Default text-output path is silent.
- Errors propagate from the function via `throw new Error('Failed to query estimation guide: <msg>')`. The CLI catches via `output.error('✗ Query failed:', err.message)` + `process.exit(1)`.
- The TS CLI `.action` signature passes `workUnitId` AND `options` but the function signature in `queryEstimationGuide` is `{cwd?: string}` — extra args are silently ignored.

### 3. Algorithm

```ts
const completedWorkUnits = Object.values(data.workUnits).filter(wu => wu.status === 'done');

const byPoints: Record<number, {iterations: number[]}> = {};
for (const wu of completedWorkUnits) {
  if (wu.estimate && wu.iterations !== undefined) {
    byPoints[wu.estimate] ??= {iterations: []};
    byPoints[wu.estimate].iterations.push(wu.iterations);
  }
}

const patterns: EstimationPattern[] = [];
for (const [pointsStr, stats] of Object.entries(byPoints)) {
  const points = parseInt(pointsStr);
  const minIterations = Math.min(...stats.iterations);
  const maxIterations = Math.max(...stats.iterations);
  let confidence = 'low';
  if (stats.iterations.length >= 4) confidence = 'high';
  else if (stats.iterations.length >= 2) confidence = 'medium';
  patterns.push({points, expectedIterations: `${min}-${max}`, confidence});
}

patterns.sort((a, b) => a.points - b.points);
return {patterns};
```

### 4. Result shape (JSON field order — declaration order)

```jsonc
{
  "patterns": [
    {
      "points": 3,
      "expectedIterations": "1-2",
      "confidence": "medium"
    }
  ]
}
```

### 5. CLI wrapper (`registerQueryEstimationGuideCommand`)

```ts
.argument('<workUnitId>', 'Work unit ID')
.option('--format <format>', 'Output format: text or json', 'text')
.action(async (workUnitId, {format}) => {
  const result = await queryEstimationGuide({workUnitId, format});  // unused fields
  if (format === 'json') output.log(JSON.stringify(result, null, 2));
});
```

### 6. Rust port targets

| Layer    | TS                                          | Rust                                                                |
|----------|---------------------------------------------|---------------------------------------------------------------------|
| Core fn  | `src/commands/query-estimation-guide.ts`    | `codelet/fspec-core/src/commands/query_estimation_guide.rs`         |
| Help cfg | `src/commands/query-estimation-guide-help.ts` | `codelet/fspec-core/src/help/configs/query_estimation_guide.rs`  |
| CLI br   | (Commander.js registration)                 | `codelet/fspec/src/query_estimation_guide.rs`                       |

### 7. WorkUnit fields consumed

- `status`, `estimate`, `iterations` — `estimate` and `iterations` are NOT modeled on Rust `WorkUnit` (live in `extra`). Read via `extra.get("estimate").and_then(Value::as_u64)` and similar.

### 8. ENOENT semantics (PARITY DECISION)

TS does NOT auto-create on missing file; it propagates the ENOENT through the `readFile` error → caught and rethrown with the `Failed to query estimation guide: ENOENT...` message.

For Rust parity:
- Implement as `read_work_units_or_empty` semantic? **NO** — TS escalates ENOENT. We need a direct read that errors on missing file.
- Decision: use `ensure_work_units_file` (auto-create) for simpler parity with our existing ecosystem; document that text path is silent so the user-facing difference is invisible for the empty-file case. This still matches the JSON path: empty file produces `{"patterns": []}`. **Framing A** — TS broken edge case (ENOENT errors out unhelpfully); Rust auto-creates the canonical empty store and returns the correct empty `{patterns: []}` JSON.

Actually, re-read TS more carefully: the function will throw "Failed to query estimation guide: ENOENT: no such file or directory ..." on a fresh project — that's a poor UX bug. The dispatcher path needs a coherent result. We will use `ensure_work_units_file` (creates the file with canonical default) and return `{patterns: []}` for the empty store. Document as Framing A divergence.

### 9. Two-front-doors invariant

Both LLM dispatcher and clap subcommand call **one** `pub async fn run(args_json, project_root)` in `fspec-core`. Bridge marshals `{workUnitId, format}` JSON and delegates. The workUnitId field is accepted but unused by the core function (TS-parity ignore).

### 10. Corner cases to test

- Empty work-units.json → patterns: [].
- No done units → patterns: [].
- One done unit with estimate=3, iterations=1 → patterns: [{3, "1-1", "low"}].
- Two done units estimate=3, iterations=[1,2] → patterns: [{3, "1-2", "medium"}].
- Four done units estimate=5, iterations=[1,2,3,4] → patterns: [{5, "1-4", "high"}].
- Done unit missing iterations → skipped from group.
- Done unit missing estimate → skipped from group.
- Non-done units → ignored entirely.
- Multiple point buckets → sorted ascending by points.
- `--format json` → pretty JSON.
- `--format text` (default) or omitted → silent stdout.
- Required positional workUnitId is parsed but unused.
- Missing `work-units.json` → auto-created (Rust Framing A) → patterns: [].
- Malformed JSON → ParseJson error.

### 11. Output rendering matrix

| `--format` | Stdout                                  | Exit code |
|-----------|------------------------------------------|-----------|
| `text` (default) | (silent — NOTHING printed)        | 0         |
| `json`    | `JSON.stringify(result, null, 2)`        | 0         |
| (error)   | stderr: `Error: <msg>`                   | 1         |

End of research.
