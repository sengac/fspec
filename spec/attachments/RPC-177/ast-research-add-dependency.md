# RPC-177 — add-dependency AST research

## TS source: `src/commands/add-dependency.ts`

Signature: `export async function addDependency(options: AddDependencyOptions): Promise<AddDependencyResult>`

```ts
interface AddDependencyOptions {
  workUnitId: string;
  blocks?: string;
  blockedBy?: string;
  dependsOn?: string;
  relatesTo?: string;
  cwd?: string;
}
interface AddDependencyResult { success: boolean; }
```

### Behaviours observed

1. `cwd = options.cwd || process.cwd()`. Project root resolution.
2. `await ensureWorkUnitsFile(cwd)` — auto-creates `spec/work-units.json` when missing.
3. **Validates SOURCE work unit exists.** If `!data.workUnits[options.workUnitId]` → throw `Error("Work unit '<id>' does not exist")`.
4. Then processes EACH relationship flag in order: `blocks → blockedBy → dependsOn → relatesTo`. All flags are independent and each is processed if provided.

### `blocks` (bidirectional, with side-effect of auto-transitioning target to blocked)

1. If target missing → `Error("Target work unit '<id>' does not exist")`.
2. If `workUnitId === blocks` → `Error("Cannot create self-dependency")`.
3. If `workUnit.blocks?.includes(target)` → `Error("Dependency already exists")`.
4. Cycle detection via `detectCircularDependency(workUnits, fromId=workUnitId, toId=blocks)`. On cycle → `Error("Circular dependency detected: <workUnitId> -> <cycle-path>")`.
5. Init `workUnit.blocks = []` if absent; push target.
6. Init `target.blockedBy = []` if absent; push source (NO duplicate guard on reverse edge — straight push).
7. **Auto-transition target to blocked** if `target.status !== 'blocked' && target.status !== 'done'`:
   - `oldStatus = target.status`
   - `target.status = 'blocked'`
   - `data.states[oldStatus] = data.states[oldStatus].filter(id => id !== target)` — drop from old array.
   - `data.states.blocked` init if absent; push target if not already present (dedup).

### `blockedBy` (bidirectional, with side-effect of auto-transitioning source to blocked)

1. If target missing → `Error("Target work unit '<id>' does not exist")`.
2. If `workUnitId === blockedBy` → `Error("Cannot create self-dependency")`.
3. If `workUnit.blockedBy?.includes(target)` → `Error("Dependency already exists")`.
4. Cycle detection from blocker's perspective: `detectCircularDependency(workUnits, fromId=blockedBy, toId=workUnitId)`. On cycle → `Error("Circular dependency detected: <blockedBy> -> <cycle-path>")`.
5. Init `workUnit.blockedBy = []`; push target.
6. Init `target.blocks = []`; push source (NO duplicate guard on reverse edge).
7. **Auto-transition SOURCE to blocked** if `workUnit.status !== 'blocked' && workUnit.status !== 'done'`:
   - `oldStatus = workUnit.status`
   - `workUnit.status = 'blocked'`
   - `workUnit.blockedReason = "Blocked by <blockedBy>"`  ← NOTE: added on source
   - States arrays updated the same way as above.

### `dependsOn` (unidirectional, no auto-transition)

1. If target missing → `Error("Target work unit '<id>' does not exist")`.
2. If `workUnitId === dependsOn` → `Error("Cannot create self-dependency")`.
3. If `workUnit.dependsOn?.includes(target)` → `Error("Dependency already exists")`.
4. Init `workUnit.dependsOn = []`; push target.
5. No reverse edge. No state transition.

### `relatesTo` (symmetric, no auto-transition, idempotent reverse-edge guard)

1. If target missing → `Error("Target work unit '<id>' does not exist")`.
2. If `workUnitId === relatesTo` → `Error("Cannot create self-dependency")`.
3. If `workUnit.relatesTo?.includes(target)` → `Error("Dependency already exists")`.
4. Init `workUnit.relatesTo = []`; push target.
5. Init `target.relatesTo = []`; push source IF `!target.relatesTo.includes(source)` — idempotent.

### Post-processing

- `workUnit.updatedAt = new Date().toISOString()` on the source unit only.
- `fileManager.transaction(workUnitsFile, async fileData => Object.assign(fileData, data))` — atomic write.
- Returns `{ success: true }`.

### Cycle detection (`detectCircularDependency`)

DFS over `blocks` adjacency. Function signature:

```ts
function detectCircularDependency(
  workUnits, fromId, toId,
  visited: Set<string> = new Set(),
  path: string[] = []
): string | null
```

- If `visited.has(toId)` → return null.
- `visited.add(toId); path.push(toId);`
- If `toId === fromId && path.length > 1` → return `path.join(' -> ')`.
- For each `blockedId of workUnits[toId]?.blocks`:
  - Recurse with `new Set(visited)` and `[...path]` (branch isolation).
  - If recursion returns a cycle → propagate it.
- Return null.

### CLI surface (Commander.js)

```ts
program
  .command('add-dependency')
  .argument('[workUnitId]', 'Work unit ID')
  .argument('[dependsOnId]', 'Work unit ID that this depends on (shorthand for --depends-on)')
  .option('--blocks <targetId>', '...')
  .option('--blocked-by <targetId>', '...')
  .option('--depends-on <targetId>', '...')
  .option('--relates-to <targetId>', '...')
  .action(...)
```

- 2 positional args (both optional in Commander), 4 flag-only options.
- Action does additional argument validation:
  - `finalDependsOn = dependsOnId || options.dependsOn`.
  - If both shorthand positional AND `--depends-on` provided AND they differ → `Error("Cannot specify dependency both as argument and --depends-on option")`.
  - If `!finalDependsOn && !options.blocks && !options.blockedBy && !options.relatesTo` → `Error("Must specify at least one relationship: <depends-on-id> or --blocks/--blocked-by/--depends-on/--relates-to")`.
- On success: `output.log('✓ Dependency added successfully')`.
- On failure: `output.error('✗ Failed to add dependency:', error.message)` + `process.exit(1)`.

### Help — `src/commands/add-dependency-help.ts`

- name: `add-dependency`
- description: "Add dependency relationships between work units to track blockers and dependencies"
- usage: `fspec add-dependency <id> [dependsOnId] [options]`
- 4 options (--blocks, --blocked-by, --depends-on, --relates-to).
- 5 examples (shorthand, --blocks, --blocked-by, --depends-on, --relates-to).
- 2 commonErrors (work unit not found, circular).
- relatedCommands: remove-dependency, dependencies, export-dependencies, clear-dependencies.

### Rust port plan

- Reuse `crate::commands::add_dependencies` machinery: `apply_blocks`, `apply_blocked_by`, `apply_depends_on`, `apply_relates_to`, `detect_cycle`, etc. — but those are private to `add_dependencies.rs`. Either duplicate or refactor into a shared module (`graph_ops`). Choice for this port: re-implement inline (small) to avoid touching a shared file; mirror semantics exactly.
- `WorkUnit` has `#[serde(flatten)] extra: Map<String, Value>`. The `blocks`, `blockedBy`, `dependsOn`, `relatesTo`, `blockedReason` fields all live in `extra`.
- Args struct: `AddDependencyArgs { workUnitId, blocks?, blockedBy?, dependsOn?, relatesTo? }`. The CLI bridge resolves the positional shorthand BEFORE marshalling JSON.
- Error message strings must match TS verbatim (substring assertions in tests).
- Result JSON: `{success: true}` only (no `added` count — TS doesn't return it).
- `iso8601_now()` for `updatedAt` on source.
- Single atomic `write_json_atomic` at the end.

### Two-front-doors

The CLI bridge owns:
- Conflict detection (positional shorthand + `--depends-on` flag).
- "At least one relationship" check.
The bridge produces a single JSON object with only the flags set; the core `run` then performs all per-flag domain logic. This mirrors how `add_dependencies` is structured.
