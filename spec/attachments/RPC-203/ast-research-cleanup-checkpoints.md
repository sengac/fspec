# AST Research — `cleanup-checkpoints` (RPC-203)

## TS sources
- `src/commands/cleanup-checkpoints.ts` (115 LOC).
- `src/commands/cleanup-checkpoints-help.ts`.
- `src/utils/git-checkpoint.ts` → `cleanupCheckpoints()` (382-417) + `listCheckpoints()` (344-377).

## Commander surface
```
program.command('cleanup-checkpoints')
  .argument('<work-unit-id>')
  .requiredOption('--keep-last <number>')
  .action(cleanupCheckpointsCommand)
```
One positional + required `--keep-last`.

## CLI wrapper validation (`cleanupCheckpointsCommand`)
- `keepLast = parseInt(opts.keepLast, 10)`.
- `if (isNaN(keepLast) || keepLast < 1) throw '--keep-last must be a positive number'` → `Error:` + exit 1.
- success → exit 0.

## Behaviour (`cleanupCheckpoints()` util)
1. `listCheckpoints(wu, cwd)`:
   - read index `.git/fspec-checkpoints-index/<wu>.json` for timestamps.
   - `listGhostCheckpoints(cwd, wu)` (codelet-git ref enumeration).
   - build `Checkpoint[]` with `timestamp = indexEntry?.timestamp ?? now`, `isAutomatic = name.includes('-auto-')`.
   - sort newest-first by timestamp.
2. `preserved = checkpoints.slice(0, keepLast)`, `deleted = checkpoints.slice(keepLast)`.
3. For each deleted: `deleteGhostCheckpoint(cwd, wu, name)` — errors swallowed (continue).
4. Returns `{ deletedCount, preservedCount, deleted[], preserved[] }`.
   NOTE: the index file is NOT pruned by `cleanupCheckpoints` (only `cleanupAutoCheckpoints`/`deleteCheckpoint` prune it). Mirror: do NOT rewrite index here. (Stale entries are tolerated — list-checkpoints intersects with live refs.)

## Command-level output (`cleanupCheckpoints()` in command file)
```
\nCleaning up checkpoints for <wu> (keeping last <keepLast>)...\n
```
then if deletedCount>0:
```
Deleted <N> checkpoint(s):
  - <name> (<timestamp>)
  ...
<blank>
```
then if preservedCount>0:
```
Preserved <N> checkpoint(s):
  - <name> (<timestamp>)
  ...
<blank>
```
then always:
```
✓ Cleanup complete: <deletedCount> deleted, <preservedCount> preserved
```
`sendIPCMessage({type:'checkpoint-changed'})` — **NO-OP in Rust**.

## Rust wiring
- `codelet_git::ghost_commit::list_ghost_checkpoints` + `delete_ghost_checkpoint`.
- Reuse list+sort logic shape from `list_checkpoints.rs` (read_index, build_display, sort by timestamp desc). Could factor a small local helper; keep it in this module to respect file-ownership rules.
- `keep_last` validation: dispatcher arg is a number; CLI bridge parses string → must be >=1 else `InvalidArgs`/exit 1.

## Args
```
{ "workUnitId": str (required), "keepLast": u32 (required, >=1), "format"?: "text"|"json" }
```

## JSON dispatcher shape
```
{ "workUnitId": str, "deletedCount": N, "preservedCount": N,
  "deleted": [{name, timestamp}], "preserved": [{name, timestamp}] }
```

## Errors
- Missing/empty workUnitId → InvalidArgs.
- keepLast < 1 or non-numeric → InvalidArgs ('--keep-last must be a positive number').
