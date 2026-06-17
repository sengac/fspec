# AST Research — `checkpoint` (RPC-202)

## TS sources
- `src/commands/checkpoint.ts` (90 LOC) — Commander.js registration + `checkpoint()` exported fn.
- `src/commands/checkpoint-help.ts` — `CommandHelpConfig` default export.
- `src/utils/git-checkpoint.ts` → `createCheckpoint()` (lines 152-198) + `updateCheckpointIndex()` (71-109).

## Commander surface
```
program.command('checkpoint')
  .argument('<work-unit-id>')
  .argument('<checkpoint-name>')
  .action(checkpointCommand)
```
NO `.option(...)` flags. Two required positionals.

## Behaviour (TS `checkpoint()`)
1. `createCheckpointUtil({ workUnitId, checkpointName, cwd, includeUntracked: true })`.
   - Internally calls Rust NAPI `createGhostCheckpoint(cwd, workUnitId, checkpointName)`.
   - If `result.files.length === 0` → returns `success: false` (clean working dir, nothing captured).
   - Else writes/updates `.git/fspec-checkpoints-index/<workUnitId>.json` with `{name, sha, timestamp: now().toISOString()}` (deduped by name).
2. On success prints:
   - `✓ Created checkpoint "<name>" for <workUnitId>`
   - `  Captured <N> file(s)`
3. `sendIPCMessage({type:'checkpoint-changed'})` — **NO-OP in Rust** (no TUI IPC in dispatcher).
4. On thrown error: `✗ Failed to create checkpoint: <msg>` then rethrows.
5. CLI wrapper `checkpointCommand`: `process.exit(0)` if success else `exit(1)`; on caught error `Error: <msg>` + `exit(1)`.

## Return shape
`{ success, checkpointName, stashMessage, includedUntracked, capturedFiles }`
- `stashMessage`: `fspec-checkpoint:<wu>:<name>:<Date.now()>`
- `stashRef`: `refs/fspec-checkpoints/<wu>/<name>` (returned by util but not by command-level fn).

## Rust wiring
- `codelet_git::ghost_commit::create_ghost_commit(project_root, &wu, &name) -> GhostCheckpoint { sha, parent_sha, files }`.
- Empty `files` ⇒ `success:false` parity (no index write).
- Index write lives in fspec-core (codelet-git does NOT touch the index): write `.git/fspec-checkpoints-index/<wu>.json`, `{ checkpoints: [{name, sha, timestamp}] }`, dedup by name, `serde_json::to_string_pretty` (2-space, matching `JSON.stringify(...,null,2)`).
- `timestamp` = ISO-8601 now (reuse the civil-time fallback formatter pattern from `list_checkpoints.rs`).
- IPC = no-op (architecture note).

## JSON dispatcher shape (`format:"json"`)
```
{ "success": bool, "checkpointName": str, "capturedFiles": [str], "includedUntracked": true }
```
`#[derive(Serialize)]` struct to preserve key order.

## Text rendering
```
✓ Created checkpoint "<name>" for <wu>
  Captured <N> file(s)
```
(empty/clean case is `success:false` — TS still returns; command-level prints nothing extra, exits 1.)

## Errors
- Missing/empty `workUnitId` or `checkpointName` → `InvalidArgs` (dispatcher strictness, parity with list-checkpoints).
- Not a git repo / create failure → util returns `success:false` (TS swallows via catch → success:false). Mirror: treat `create_ghost_commit` Err as `success:false` (no throw), exit 1.
