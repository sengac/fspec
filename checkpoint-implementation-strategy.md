# Git Checkpoint Implementation Strategy

## Problem Analysis

**Current Implementation:** Uses `git.commit()` to simulate stashing, but never stages files with `git.add()`, resulting in empty commits that capture nothing.

**Root Cause:** Implementation ignores isomorphic-git's native `git.stash()` API.

## isomorphic-git Stash API Research

### Key Findings

1. **isomorphic-git HAS full `git.stash()` support** with operations:
   - `push`: Create stash, clean working directory
   - `apply`: Restore stash changes
   - `pop`: Apply + drop stash
   - `drop`: Delete stash entry
   - `list`: List all stashes
   - `clear`: Remove all stashes
   - `create`: **Create stash commit WITHOUT modifying working directory or refs**

2. **Critical Limitation:** isomorphic-git stash only handles **TRACKED files**
   - Modified tracked files: ✅ Stashed
   - Staged tracked files: ✅ Stashed
   - Untracked new files: ❌ NOT stashed
   - This is documented in the API: "all stash operations are done on tracked files only"

3. **Test Evidence:**
   - Test: "stash with untracked files - no other changes" → Error: "nothing to stash"
   - Test: "stash with untracked files - with other changes" → Untracked files remain unchanged

## The Winning Strategy: `git.stash({ op: 'create' })` + Pre-staging

Use isomorphic-git's `create` operation, which creates a stash commit but doesn't modify working directory or refs.

### Implementation

```typescript
async function createCheckpoint(options: CheckpointOptions) {
  const { workUnitId, checkpointName, cwd } = options;

  // 1. Get all changed files (tracked + untracked)
  const status = await git.statusMatrix({ fs, dir: cwd });
  const changedFiles = status.filter(row => {
    const [, headStatus, workdirStatus, stageStatus] = row;
    return headStatus !== workdirStatus || workdirStatus !== stageStatus;
  }).map(row => row[0]);

  if (changedFiles.length === 0) {
    return { success: false, reason: 'No changes to checkpoint' };
  }

  // 2. Stage ALL files (including untracked) so stash can see them
  for (const filepath of changedFiles) {
    await git.add({ fs, dir: cwd, filepath });
  }

  // 3. Create stash commit WITHOUT modifying workdir/refs
  const message = createCheckpointMessage(workUnitId, checkpointName);
  const stashOid = await git.stash({
    fs,
    dir: cwd,
    op: 'create',  // KEY: doesn't touch workdir or refs!
    message
  });

  // 4. Store in custom ref namespace (keeps checkpoints organized)
  await git.writeRef({
    fs,
    dir: cwd,
    ref: `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`,
    value: stashOid
  });

  // 5. Reset index to avoid polluting user's staging area
  await git.resetIndex({ fs, dir: cwd });

  return {
    success: true,
    checkpointOid: stashOid,
    capturedFiles: changedFiles
  };
}
```

### Restore Implementation

```typescript
async function restoreCheckpoint(options: RestoreOptions) {
  const { workUnitId, checkpointName, cwd } = options;

  // 1. Read checkpoint commit OID from custom ref
  const checkpointOid = await git.resolveRef({
    fs,
    dir: cwd,
    ref: `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`
  });

  // 2. Get all files in checkpoint commit
  const { commit } = await git.readCommit({
    fs,
    dir: cwd,
    oid: checkpointOid
  });

  // 3. Read tree and restore each file to working directory
  const tree = commit.tree;
  const files = await git.listFiles({
    fs,
    dir: cwd,
    ref: checkpointOid
  });

  for (const filepath of files) {
    const { blob } = await git.readBlob({
      fs,
      dir: cwd,
      oid: checkpointOid,
      filepath
    });

    const fullPath = join(cwd, filepath);
    await fs.promises.mkdir(dirname(fullPath), { recursive: true });
    await fs.promises.writeFile(fullPath, blob);
  }

  return {
    success: true,
    restoredFiles: files
  };
}
```

### List Checkpoints

```typescript
async function listCheckpoints(workUnitId: string, cwd: string) {
  const checkpointRefs = await git.listRefs({
    fs,
    dir: cwd,
    prefix: `refs/fspec-checkpoints/${workUnitId}/`
  });

  const checkpoints: Checkpoint[] = [];

  for (const ref of checkpointRefs) {
    const checkpointName = ref.ref.replace(`refs/fspec-checkpoints/${workUnitId}/`, '');
    const { commit } = await git.readCommit({
      fs,
      dir: cwd,
      oid: ref.oid
    });

    const parsed = parseCheckpointMessage(commit.message);
    checkpoints.push({
      name: checkpointName,
      workUnitId,
      timestamp: new Date(parseInt(parsed.timestamp)).toISOString(),
      stashRef: ref.ref,
      isAutomatic: checkpointName.startsWith(`${workUnitId}-auto-`),
      message: commit.message
    });
  }

  return checkpoints;
}
```

### Cleanup Checkpoints

```typescript
async function cleanupCheckpoints(
  workUnitId: string,
  cwd: string,
  keepLast: number
) {
  const checkpoints = await listCheckpoints(workUnitId, cwd);

  // Sort by timestamp (newest first)
  checkpoints.sort((a, b) =>
    new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
  );

  const preserved = checkpoints.slice(0, keepLast);
  const deleted = checkpoints.slice(keepLast);

  // Delete old checkpoint refs
  for (const checkpoint of deleted) {
    await git.deleteRef({
      fs,
      dir: cwd,
      ref: checkpoint.stashRef
    });
  }

  return {
    deletedCount: deleted.length,
    preservedCount: preserved.length,
    deleted,
    preserved
  };
}
```

## Why This Solution Works

✅ **Uses native `git.stash()`** - No commit pollution in git history
✅ **Captures untracked files** - By staging them first with `git.add()`
✅ **Preserves working directory** - `op: 'create'` doesn't touch it
✅ **Custom ref storage** - `.git/refs/fspec-checkpoints/` keeps organized
✅ **Clean index** - Resets after checkpoint to avoid staging pollution
✅ **Git-native** - All data in git object database, inspectable with git commands

## Key Differences from Current Implementation

| Current (Broken) | New (Correct) |
|-----------------|---------------|
| Uses `git.commit()` directly | Uses `git.stash({ op: 'create' })` |
| Never stages files | Stages all files before stashing |
| Pollutes git commit history | Uses stash commits (separate from history) |
| Doesn't reset index | Resets index after checkpoint |
| Hard to list/manage | Custom ref namespace for organization |

## Migration Path

1. Fix `createCheckpoint()` in `src/utils/git-checkpoint.ts`
2. Fix `restoreCheckpoint()` to read from refs and restore files
3. Fix `listCheckpoints()` to read from custom ref namespace
4. Update tests to verify untracked files are captured
5. Update feature specification if needed (document staging requirement)

## Testing Strategy

- Test with modified tracked files only
- Test with new untracked files only
- Test with both tracked + untracked files
- Test checkpoint restoration
- Test multiple checkpoints per work unit
- Test automatic cleanup of old checkpoints
