# AST Research: `query-estimate-accuracy` (RPC-258)

## TS Source of Truth

- `src/commands/query-estimate-accuracy.ts` (218 lines)
- `src/commands/query-estimate-accuracy-help.ts` (26 lines)

## Behavioural Surface Mapped from TypeScript

### Inputs (`queryEstimateAccuracy()` function)

```ts
options: {
  workUnitId?: string;   // when present, returns SingleWorkUnitAccuracy
  byPrefix?: boolean;    // when true (and no workUnitId), adds `byPrefix` field
  output?: string;       // unused in current TS path (the CLI passes `format`, not `output`)
  cwd?: string;          // project root override (default process.cwd())
}
```

### CLI Surface (`registerQueryEstimateAccuracyCommand`, lines 164–217)

- Commander registers ONLY `--format <format>` flag (default `'text'`).
- It does NOT register `--prefix` (advertised in help-config) nor `--work-unit-id` nor `--by-prefix`.
- This is a TS bug: the CLI cannot trigger `workUnitId` or `byPrefix` paths. The CLI invocation always calls `queryEstimateAccuracy({ format: options.format })` (line 171) — passing a `format` key that the function does NOT consume.
- → For the Rust port, the **CLI** surface is "no workUnitId / no byPrefix; format=text|json"; the **dispatcher** surface needs to expose `workUnitId`, `byPrefix`, and `format` to honour the TS `queryEstimateAccuracy` function shape.

### File I/O

- Reads `spec/work-units.json` via `readFile(...)` then `JSON.parse(...)`.
- ENOENT or parse-error → wrapped in a `try/catch` that re-throws `Error("Failed to query estimate accuracy: <message>")` (lines 156–161).
- No auto-create; no escape on partially-malformed entries.

### Single-Work-Unit Path (lines 56–74)

- If `options.workUnitId`:
  - Look up `data.workUnits[id]`. If missing → throw `Error("Work unit <id> not found")` (line 59). That error is then wrapped by the outer catch into `"Failed to query estimate accuracy: Work unit <id> not found"`.
  - Iterations source = `workUnit.iterations || workUnit.metrics?.iterations || 0`.
  - Returns `{ estimated: "<estimate||0> points", actual: "0 tokens, <iterations> iterations", comparison: "Within expected range" }`.

### All-Completed Path (lines 77–155)

- Filter `Object.values(workUnits)` where `status === 'done'`.
- Iterations source for each = `wu.iterations || wu.metrics?.iterations` (no `|| 0` — falsy is filtered out by the `iterations !== undefined` check below).
- For each completed wu with `wu.estimate && iterations !== undefined`:
  - Aggregate into `byStoryPoints[estimate.toString()] = { totalIterations, count }`.
- Output `avgIterations = Math.round((totalIterations/count) * 10) / 10` (one decimal).
- Output `samples = count`.
- Iteration order = `Object.entries(byStoryPoints)` (insertion order of first encounter in `Object.values(data.workUnits)`).

### Prefix Path (lines 114–151)

- Only when `options.byPrefix === true`.
- For each completed wu with `wu.estimate && iterations !== undefined`:
  - `prefix = wu.id.split('-')[0]`
  - Bump `byPrefix[prefix] = { totalIterations, count }`.
- Output per prefix:
  - `avgAccuracy = "${avgIterations.toFixed(1)} avg iterations"` (string).
  - `recommendation = "${count} sample${count>1?'s':''}"` (string).
- Returned alongside `byStoryPoints`.

### CLI Text Renderer (lines 178–212)

- Leading: `output.log('\n📊 Estimation Accuracy Report\n');`
- If `Object.keys(byStoryPoints).length === 0`:
  - `"No completed work units with estimates and actuals found."`
  - Followed by guidance: `"\nTo track accuracy, work units need:\n  • Status: done\n  • estimate field (story points)\n  • iterations field\n"`
  - Then `return;` — does NOT emit any trailing prefix breakdown.
- Else:
  - `"By Story Points:"`
  - For each entry: `"\n  ${points} points:\n    Average iterations: ${metrics.avgIterations}\n    Samples: ${metrics.samples}"`
  - If `data.byPrefix`:
    - `"\n\nBy Prefix:"`
    - For each: `"\n  ${prefix}:\n    Accuracy: ${accuracy.avgAccuracy}\n    Recommendation: ${accuracy.recommendation}"`
  - Trailing empty `output.log();`.

### Help Config (`query-estimate-accuracy-help.ts`)

- `name = "query-estimate-accuracy"`
- `description = "Show estimation accuracy metrics comparing estimates to actuals"`
- `usage = "fspec query-estimate-accuracy [options]"`
- `whenToUse = "Use to assess estimation accuracy and improve future estimates."`
- `options = [{ flag: '--prefix <prefix>', description: 'Filter by prefix' }]` (help-only; not actually wired)
- `examples = [{ command: 'fspec query-estimate-accuracy', description: 'Show accuracy metrics', output: 'Average estimate accuracy: 87%\nUnderestimated: 5 work units\nOverestimated: 3 work units' }]`
- `relatedCommands = ['update-work-unit-estimate', 'query-metrics']`

### Captured Help Fixture

Running `node dist/index.js query-estimate-accuracy --help` produces (verbatim):

```
\n
QUERY-ESTIMATE-ACCURACY
Show estimation accuracy metrics comparing estimates to actuals

WHEN TO USE
  Use to assess estimation accuracy and improve future estimates.

USAGE
  fspec query-estimate-accuracy [options]

OPTIONS
  --prefix <prefix>
    Filter by prefix

EXAMPLES
  1. Show accuracy metrics
  $ fspec query-estimate-accuracy
  Average estimate accuracy: 87%
Underestimated: 5 work units
Overestimated: 3 work units

RELATED COMMANDS
  fspec update-work-unit-estimate
  fspec query-metrics
```

## Existing Rust Infrastructure

- `crate::io::ensure::read_work_units_or_empty(project_root)` — returns `WorkUnitsData` with ENOENT → empty, parse-error → escalated.
- BUT: TS's behaviour wraps parse errors via the outer try/catch into `"Failed to query estimate accuracy: ..."`. We may need to special-case the parse error message OR re-read the raw file when work-unit-id is requested. For dispatcher path, the cleanest mapping is to let `read_work_units_or_empty` escalate as `ParseJson` and have `run` translate it to `InvalidArgs` with the canonical wrapper string.
- `crate::types::work_unit::WorkUnit` — already supports `estimate` (Option<f64> or Option<u32>). We need to verify `iterations` and `metrics.iterations` are exposed.

## Risk / Edge Notes

1. The TS function field-shape uses Math.round semantics (× 10 / 10 = 1 decimal). Rust must replicate via `(x * 10.0).round() / 10.0`.
2. Order of `byStoryPoints` keys = `Object.keys()` insertion order = order of FIRST encounter scanning `Object.values(data.workUnits)`. Use an `IndexMap<String, ...>` aggregator OR a `Vec<(String, ...)>` to preserve order on the wire.
3. Order of `byPrefix` keys = same insertion-order rule applied to `wu.id.split('-')[0]`.
4. JSON output uses `JSON.stringify(result, null, 2)` — pretty-printed with 2-space indent and key declaration order preserved (`byStoryPoints` then `byPrefix`).
5. The TS catch wraps everything via `"Failed to query estimate accuracy: <message>"` — including "Work unit X not found". For Rust dispatcher parity, surface the wrapped form via `FspecCoreError::InvalidArgs { reason }`.
6. The single-work-unit branch does NOT require `status === 'done'`. It just looks up by id.
7. CLI today does NOT pass `workUnitId` or `byPrefix` flags — only `--format`. We mirror this exactly in clap: the CLI surface is `--format text|json` only.

## TS Test Coverage (existing)

`src/commands/__tests__/work-unit-estimation-and-metrics.test.ts`:
- Single work-unit path (AUTH-001 with estimate=5, iterations=2 → `"5 points"`, `"0 tokens, 2 iterations"`).
- All completed (mixed estimates 1/1/3/3/5).
- `byPrefix=true` over AUTH-* and SEC-*.

## Conclusion

Port can reuse `read_work_units_or_empty` plus a thin aggregator (≈100 LOC). The help fixture is identical TS-style; the CLI exposes only `--format`. The dispatcher exposes `workUnitId`, `byPrefix`, `format`.
