# RPC-257 — AST Research: `query-dependency-stats`

## Source of truth (TypeScript)

- Implementation: `src/commands/query-dependency-stats.ts`
- Help config: `src/commands/query-dependency-stats-help.ts`

## TS behavioural inventory

### 1. Inputs
- Constructor option: `cwd?: string` (defaults to `process.cwd()`).
- Loads `spec/work-units.json` via `ensureWorkUnitsFile(cwd)` — **auto-creates** the file with the canonical initial structure if missing.
- CLI flag `--format <text|json>` (default `text`). The function itself does NOT consume `format`; the CLI wrapper uses it to decide whether to `JSON.stringify(result)` to stdout.

### 2. Aggregation pass (`for workUnit of Object.values(data.workUnits)`)

For each work unit, increments:

| Field on WU                     | Counter incremented by N        | Per-unit boolean tally bumped     |
|---------------------------------|---------------------------------|------------------------------------|
| `blocks?.length` (truthy)       | `totalBlocks += N`              | `workUnitsBlockingOthers++`       |
| `blockedBy?.length` (truthy)    | `totalBlockedBy += N`           | `workUnitsWithBlockers++`         |
| `dependsOn?.length` (truthy)    | `totalDependsOn += N`           | `workUnitsWithSoftDependencies++` |
| `relatesTo?.length` (truthy)    | `totalRelatesTo += N`           | (no dedicated bucket)             |

If **any** of the four arrays is non-empty on a WU, increment `workUnitsWithDependencies` once.

### 3. Average

```
totalDependencies = totalBlocks + totalBlockedBy + totalDependsOn + totalRelatesTo
averageDependenciesPerUnit = workUnitArray.length > 0
  ? totalDependencies / workUnitArray.length
  : 0
// Rounded to 2 decimals via Math.round(x*100)/100
```

Empty-workspace divisor protection: explicit `length > 0` guard.

### 4. Max dependency chain depth

DFS through `wu.blocks` recursively from every WU as root:
- Each call carries a `visited: Set<string>` (cloned per branch — `new Set(visited)` on each recursion).
- If visiting an already-visited node, return 0 (cycle break).
- If WU doesn't exist (broken ref), return 0.
- `maxChildDepth = max over wu.blocks of calculateDepth(child)`.
- Return value: `maxChildDepth + (wu.blocks?.length > 0 ? 1 : 0)`.
- Top level: `maxDepth = max over every WU id of calculateDepth(wuId)`.

Key edge cases:
- Empty workspace → `0`.
- WU with `blocks=[]` → depth `0` (length-0 guard).
- A → B → C linear chain (A blocks B, B blocks C) → depth `2` (two edges).
- Self-cycle (A blocks A) → depth `0` (visited-set blocks immediately on the recursive entry; the outer call returns `0 + 1 = 1` actually — let me re-trace):

  ```
  calculateDepth(A, ∅) → visited={A}, wu=A, wu.blocks=[A]
    for A: childDepth = calculateDepth(A, {A})
      visited.has(A) → return 0
    maxChildDepth = 0
    return 0 + (wu.blocks.length > 0 ? 1 : 0) = 1
  ```

  So a self-cycle returns depth **1**, not 0. Cycles between two units (A↔B):
  - `calculateDepth(A, ∅)` → visited={A}, blocks=[B]
    - `calculateDepth(B, {A})` → visited={A,B}, blocks=[A]
      - `calculateDepth(A, {A,B})` → visited.has(A) → 0
      - returns 0 + 1 = 1
    - childDepth=1, returns 1+1=2

  So A↔B cycle returns depth **2** from A's perspective, also 2 from B's perspective.

### 5. Result shape (JSON field order — declaration order)

```jsonc
{
  "totalBlocks": 0,
  "totalBlockedBy": 0,
  "totalDependsOn": 0,
  "totalRelatesTo": 0,
  "workUnitsWithDependencies": 0,
  "workUnitsWithBlockers": 0,
  "workUnitsBlockingOthers": 0,
  "workUnitsWithSoftDependencies": 0,
  "averageDependenciesPerUnit": 0,
  "maxDependencyChainDepth": 0
}
```

### 6. CLI wrapper (`registerQueryDependencyStatsCommand`)

```ts
.option('--format <format>', 'Output format: text or json', 'text')
.action(async ({format}) => {
  const result = await queryDependencyStats({format});
  if (format === 'json') output.log(JSON.stringify(result, null, 2));
});
```

Surprising behaviours:
- **Text format prints NOTHING**. The TS code only emits output when `format==='json'`. Default text path is a silent no-op (likely a TS bug; replicating for parity).
- Errors caught → `output.error('✗ Query failed:', err.message)` → `process.exit(1)`.
- Help advertises `--show-critical-path` but Commander.js registration does NOT declare it (only `--format`).

### 7. WorkUnit fields consumed (Rust shape requirements)

Rust `WorkUnit` struct currently only types: id/title/type/status/epic/createdAt/updatedAt + `extra` flatten map. The dependency arrays live in `extra`. We need to either:
- (a) Add typed `blocks/blockedBy/dependsOn/relatesTo: Option<Vec<String>>` fields to `WorkUnit`.
- (b) Read them dynamically from `extra` via `extra.get("blocks").and_then(Value::as_array)`.

**Decision: (b) extra lookup** — same approach used by other ported commands so we don't bloat the shared `WorkUnit` type. This keeps changes inside `commands/query_dependency_stats.rs` and avoids touching the shared `types/work_unit.rs`.

### 8. Rust port targets (file-by-file)

| Layer    | TS                                          | Rust                                                             |
|----------|---------------------------------------------|------------------------------------------------------------------|
| Core fn  | `src/commands/query-dependency-stats.ts`    | `codelet/fspec-core/src/commands/query_dependency_stats.rs`      |
| Help cfg | `src/commands/query-dependency-stats-help.ts` | `codelet/fspec-core/src/help/configs/query_dependency_stats.rs` |
| CLI br   | (Commander.js registration)                 | `codelet/fspec/src/query_dependency_stats.rs`                    |

### 9. Two-front-doors invariant

Both the LLM dispatcher and the clap subcommand call **one** `pub async fn run(args_json, project_root)` in `fspec-core`. CLI bridge marshals `{format}` JSON and delegates — zero aggregation logic in the bridge.

### 10. Math.round → Rust

`Math.round(x*100)/100` rounds half-away-from-zero (positive only here):

```rust
let rounded = ((avg * 100.0) + 0.5).floor() / 100.0;
```

For exact f64 representation, multiply, add 0.5, floor, divide.

### 11. Glossary of corner cases to test

- Empty `work-units.json` (no entries) → average=0, depth=0, all counters=0.
- Missing file → auto-created → all zero (parity with `ensureWorkUnitsFile`).
- WU with `blocks: []` (empty array) → no counter bump (TS `?.length` is falsy on 0).
- WU with `blocks: ["NONEXISTENT"]` → counted in `totalBlocks=1`, depth recursion returns 0 for the missing child, so chain depth from this WU = 1.
- Self-cycle: depth = 1.
- A↔B cycle: depth = 2.
- A→B→C linear: depth from A = 2.
- Multiple WUs sharing the same blockedBy: counted per WU.
- A single WU with all four arrays populated: `workUnitsWithDependencies++` ONCE, but per-array buckets each bump.

### 12. Output rendering matrix (CLI surface)

| `--format` | Stdout                                  | Exit code |
|------------|-----------------------------------------|-----------|
| `text` (default) | (silent — NOTHING printed)        | 0         |
| `json`     | `JSON.stringify(result, null, 2)`       | 0         |
| (error)    | stderr: `Error: <msg>`                  | 1         |

We will replicate the silent-text behaviour for byte-parity, but at the **dispatcher** path (LLM-facing) we must still return the result. The dispatcher always gets JSON (it serialises through serde anyway). The CLI bridge handles the rendering decision.

---

## Implementation skeleton (Phase C preview)

```rust
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: Args = serde_json::from_str(args_json)?;
    let data = ensure_work_units_file(project_root)?;
    let stats = aggregate(&data.work_units);
    match args.format.as_deref() {
        Some("text") => Ok(String::new()),   // TS parity: silent text path
        _ => serde_json::to_string_pretty(&stats),
    }
}
```

End of research.
