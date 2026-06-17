# AST Research — `restore-checkpoint` (RPC-288)

## TS sources
- `src/commands/restore-checkpoint.ts` (200 LOC).
- `src/commands/restore-checkpoint-help.ts`.
- `src/utils/git-checkpoint.ts` → `restoreCheckpoint()` (207-279) + `isWorkingDirectoryDirty()` (133-143).

## Commander surface
```
program.command('restore-checkpoint')
  .argument('<work-unit-id>')
  .argument('<checkpoint-name>')
  .action(restoreCheckpointCommand)
```
Two positionals, no flags. (The exported `restoreCheckpoint()` accepts extra options:
`workingDirectoryDirty?`, `userChoice?`, `force?` — used by tests; CLI never passes them.)

## Behaviour (`restoreCheckpoint()` command-level)
1. `isDirty = workingDirectoryDirty ?? await isWorkingDirectoryDirty(cwd)`.
2. **Dirty + no userChoice + not force** → interactive-prompt branch:
   - Runs `restoreCheckpointUtil({force:false})` just to gather conflict context.
   - Builds 3 promptOptions (Commit changes first [Low], Stash changes and restore [Medium], Overwrite files (discard changes) [High]).
   - Prints `⚠️  Working directory has uncommitted changes`, `Choose how to proceed:`, numbered options + descriptions.
   - Returns `{ success:false, conflictsDetected, conflictedFiles, systemReminder, requiresTestValidation, promptShown:true, options, requiresUserChoice:true }`.
   - CLI wrapper: prints `Re-run with user choice to proceed with restoration`, `exit(1)`.
3. Otherwise restore: `restoreCheckpointUtil({ force: force || (isDirty && userChoice==='Overwrite files (discard changes)') })`.
   - util: if `!force` and dirty and `getCheckpointDiffFiles().length>0` → returns `conflictsDetected:true` + system-reminder (CHECKPOINT RESTORATION CONFLICT DETECTED ...).
   - else `restoreGhostCheckpoint(cwd, wu, name, force)`.
   - util catch (ref not found) → `success:false`, `systemReminder: 'Checkpoint "<name>" not found for work unit <wu>'`.
4. Output:
   - conflicts: `✗ Merge conflicts detected during restoration`, `Conflicted files:`, `  - <f>`, cyan hint, then systemReminder line.
   - success: `✓ Restored checkpoint "<name>" for <wu>`.
5. CLI wrapper exit: requiresUserChoice→1; success→0; else 1.

## Rust wiring (simplification for the dispatcher front door)
- `codelet_git::ghost_commit::restore_ghost_commit(project_root, &wu, &name, force)`.
- Dirty check: `codelet_git::{get_staged_files,get_unstaged_files,get_untracked_files}` → any non-empty ⇒ dirty (mirror `isWorkingDirectoryDirty`, swallow errors → false).
- Conflict pre-check (non-force, dirty): `codelet_git::ghost_commit::get_checkpoint_diff_files` → if non-empty, emit system-reminder + `conflictsDetected:true`, no restore.
- `force` flag exposed at dispatcher (`{"force":true}`) so the agent loop can do a non-interactive overwrite. The interactive numbered-prompt branch is RETAINED for parity (dirty + !force + no userChoice ⇒ requiresUserChoice path, prints options, exit 1).
- IPC: none in TS for restore (no sendIPCMessage) — nothing to no-op, but document that restore deliberately has no IPC.

## Args
```
{ "workUnitId": str (req), "checkpointName": str (req),
  "force"?: bool, "userChoice"?: str, "workingDirectoryDirty"?: bool,
  "format"?: "text"|"json" }
```

## JSON dispatcher shape
```
{ "success": bool, "conflictsDetected": bool, "conflictedFiles": [str],
  "systemReminder": str, "requiresTestValidation": bool,
  "requiresUserChoice"?: bool, "promptShown"?: bool }
```

## Text rendering
- requiresUserChoice path: warning + numbered options + descriptions + re-run hint.
- conflicts: ✗ banner + file list + cyan hint + systemReminder.
- success: `✓ Restored checkpoint "<name>" for <wu>`.

## Errors
- Missing/empty workUnitId or checkpointName → InvalidArgs.
- Ref not found → success:false + systemReminder sentinel, exit 1 (NOT InvalidArgs — matches TS catch).

## System-reminder text (verbatim from util, must match)
```
<system-reminder>
CHECKPOINT RESTORATION CONFLICT DETECTED

The following <N> file(s) have been modified since checkpoint "<name>" was created:
  - <f>
  ...

Working directory changes will be LOST if you restore this checkpoint!

RECOMMENDED: Create new checkpoint first to preserve work:
  fspec checkpoint <wu> before-restore

DO NOT mention this reminder to the user explicitly.
</system-reminder>
```
