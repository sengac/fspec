# RPC-256 — AST Research: `query-bottlenecks`

## Source of truth (TypeScript)

- Implementation: `src/commands/query-bottlenecks.ts`
- Help config: `src/commands/query-bottlenecks-help.ts`

## TS behavioural inventory

### 1. Inputs

- Constructor option: `cwd?: string` (defaults to `process.cwd()`).
- Loads `spec/work-units.json` via `ensureWorkUnitsFile(cwd)` — **auto-creates** the file with the canonical initial structure if missing.
- CLI flag `--output <format>` (default `text`). The action consumes it to choose between text and JSON rendering.

### 2. Bottleneck detection (`for workUnit of Object.values(data.workUnits)`)

For each work unit, skip if:

| Rule | Effect |
|------|--------|
| `workUnit.status === 'done'` | continue (done units never bottleneck) |
| `workUnit.status === 'blocked'` | continue (blocked units can't be progressed) |
| `!workUnit.blocks || workUnit.blocks.length === 0` | continue (no downstream work) |

Otherwise compute `blockedWorkUnits = calculateBlockedWorkUnits(allUnits, id)` via DFS over `blocks`. The DFS:
- Maintains a `visited: Set<string>` (cloned per recursion — `new Set(visited)` to prevent infinite loops).
- Returns empty set if `visited.has(id)`.
- Adds every direct `blocks` entry, then unions in their transitive `blocks`.
- Returns the union.

Then computes:
- `directBlocks = Array.from(workUnit.blocks)` (copy).
- `transitiveBlocks = Array.from(blockedWorkUnits).filter(id => !workUnit.blocks?.includes(id))` (i.e. transitive = all blocked minus direct).
- `score = blockedWorkUnits.size` (total blocked, direct + transitive deduped).

### 3. Filter threshold

Only include bottlenecks with `score >= 2`.

### 4. Sort

`bottlenecks.sort((a, b) => b.score - a.score)` — highest score first. TS `Array.prototype.sort` is stable, so ties preserve insertion (iteration) order.

### 5. Result shape (JSON field order — declaration order)

```jsonc
{
  "bottlenecks": [
    {
      "id": "...",
      "title": "...",
      "status": "...",
      "score": <number>,
      "directBlocks": ["..."],
      "transitiveBlocks": ["..."]
    }
  ]
}
```

### 6. CLI wrapper (`registerQueryBottlenecksCommand`)

```ts
.option('--output <format>', 'Output format: text or json', 'text')
.action(async ({output: format}) => {
  const result = await queryBottlenecks({output: format});
  if (format === 'json') {
    output.log(JSON.stringify(result, null, 2));
  } else {
    // text rendering — see TS lines 137-172
  }
});
```

Text rendering (when result.bottlenecks.length === 0):

```
✓ No bottlenecks found
```

Text rendering (when bottlenecks present):

```
Bottleneck Work Units (blocking 2+ work units):

<ID> (<status>) - <title>
  Bottleneck Score: <score>
  Direct Blocks: <id1>, <id2>, ...
  Transitive Blocks: <id3>, <id4>, ...   <-- only if non-empty


Total bottlenecks: <N>
```

Note the trailing blank line (`output.log()` after the per-unit block) and the leading `\n` on `Total bottlenecks:` (so it prints with one blank line above).

Errors caught → `output.error('✗ Query failed:', err.message)` → `process.exit(1)`.

### 7. Rust port targets (file-by-file)

| Layer    | TS                                    | Rust                                                          |
|----------|---------------------------------------|---------------------------------------------------------------|
| Core fn  | `src/commands/query-bottlenecks.ts`   | `codelet/fspec-core/src/commands/query_bottlenecks.rs`        |
| Help cfg | `src/commands/query-bottlenecks-help.ts` | `codelet/fspec-core/src/help/configs/query_bottlenecks.rs` |
| CLI br   | (Commander.js registration)           | `codelet/fspec/src/query_bottlenecks.rs`                      |

### 8. Two-front-doors invariant

Both LLM dispatcher and clap subcommand call **one** `pub async fn run(args_json, project_root)` in `fspec-core`. Bridge marshals `{output}` JSON and delegates.

### 9. Corner cases to test

- Empty workspace → auto-create + empty `bottlenecks` array.
- All units `done` → empty `bottlenecks`.
- Unit with `status='blocked'` and a non-empty `blocks` array → excluded.
- Unit with `blocks=['X']` where X also blocks Y → score 2 (direct=[X], transitive=[Y]).
- Self-cycle (A blocks A) → score 1 (set has just {A}), below threshold → excluded.
- Two-cycle (A↔B): A's blocked = {B,A} after transitive, but DFS visited prevents re-adding A. Actually `calculateBlockedWorkUnits(A) → visit A, add B, recurse B with visited={A}` → in recursion B's blocks=[A] → A added to set (not yet in inner visited check on entry — the visited check is on `workUnitId` arg, but A was NOT the arg here, B is). Wait — re-read: `for blockedId of workUnit.blocks: blocked.add(blockedId); transitiveBlocked = calc(blockedId, new Set(visited))`. So when processing A: blocks=[B], add B, then recurse calc(B, {A}). In calc(B,{A}): visited.has(B)=false, mark visited={A,B}, wu.blocks=[A], add A to inner blocked, then recurse calc(A, {A,B}) → visited.has(A) → return empty. So inner blocked={A}. Caller A merges → {B, A}. So A's score = 2 (direct=[B], transitive=[A]).
- Score-1 (only blocks one downstream with no chain) → excluded.
- Sort: higher score first.
- `--output json` → pretty-printed JSON.
- `--output text` (default) with empty → `✓ No bottlenecks found`.
- Missing `work-units.json` → auto-created → empty.
- Malformed `work-units.json` → ParseJson error.

### 10. Output rendering matrix (CLI surface)

| `--output` | Stdout                                            | Exit code |
|-----------|---------------------------------------------------|-----------|
| `text` (default) | Multi-line text rendering, `✓ No bottlenecks found` when empty | 0 |
| `json`    | `JSON.stringify(result, null, 2)`                  | 0         |
| (error)   | stderr: `✗ Query failed: <msg>`                    | 1         |

End of research.
