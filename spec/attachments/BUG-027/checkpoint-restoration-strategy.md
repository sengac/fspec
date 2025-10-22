# Checkpoint Restoration Strategy

## Problem Analysis

**Current Implementation Status:**
- Line 190 in `git-checkpoint.ts`: "Simulate restoration (in real implementation, would use git checkout/merge)"
- Lines 210-213: `detectConflicts()` hardcoded to return `conflicted = false`
- **The restore function is a COMPLETE STUB - it does nothing!**

This is Bug #2 of BUG-027 (Bug #1 was the creation using `git.commit()` instead of `git.stash()`).

## Why We Can't Use `git.stash({ op: 'apply' })`

**Technical Limitation:**
- isomorphic-git's `_stashApply()` expects stashes in standard `.git/refs/stash` reflog format
- It uses index-based access: `stash@{0}`, `stash@{1}`, etc.
- Our checkpoints use custom refs: `refs/fspec-checkpoints/{work-unit-id}/{checkpoint-name}`
- No clean way to make our custom refs work with native stash apply

**Conclusion:** We must implement manual file restoration.

## Restoration Strategy: Manual File Restoration with Smart Conflict Detection

### High-Level Flow

```
1. Verify checkpoint exists (resolve custom ref)
2. Check if working directory is dirty
3. Read checkpoint commit and file tree
4. Detect conflicts by comparing file contents
5. If conflicts: emit system-reminder, don't modify files
6. If no conflicts: restore all files from checkpoint
```

### Implementation

```typescript
export async function restoreCheckpoint(options: RestoreOptions): Promise<{
  success: boolean;
  conflictsDetected: boolean;
  conflictedFiles: string[];
  systemReminder: string;
  requiresTestValidation: boolean;
  restoredFiles?: string[];
}> {
  const { workUnitId, checkpointName, cwd, force = false } = options;

  // ============================================================
  // STEP 1: Check if working directory is dirty
  // ============================================================
  const isDirty = await isWorkingDirectoryDirty(cwd);

  if (isDirty && !force) {
    // Return early - interactive prompt handled in restore-checkpoint.ts command
    return {
      success: false,
      conflictsDetected: false,
      conflictedFiles: [],
      systemReminder: 'Working directory has uncommitted changes',
      requiresTestValidation: false,
    };
  }

  // ============================================================
  // STEP 2: Read checkpoint commit from custom ref
  // ============================================================
  let checkpointOid: string;
  try {
    checkpointOid = await git.resolveRef({
      fs,
      dir: cwd,
      ref: `refs/fspec-checkpoints/${workUnitId}/${checkpointName}`,
    });
  } catch (error) {
    return {
      success: false,
      conflictsDetected: false,
      conflictedFiles: [],
      systemReminder: `Checkpoint "${checkpointName}" not found for work unit ${workUnitId}`,
      requiresTestValidation: false,
    };
  }

  // ============================================================
  // STEP 3: Get checkpoint commit and list all files in tree
  // ============================================================
  const { commit } = await git.readCommit({
    fs,
    dir: cwd,
    oid: checkpointOid,
  });

  const checkpointFiles = await git.listFiles({
    fs,
    dir: cwd,
    ref: checkpointOid,
  });

  // ============================================================
  // STEP 4: Detect conflicts BEFORE modifying any files
  // ============================================================
  const conflicts: string[] = [];

  for (const filepath of checkpointFiles) {
    const fullPath = join(cwd, filepath);

    // Check if file exists in working directory
    let fileExists = false;
    try {
      await fs.promises.access(fullPath);
      fileExists = true;
    } catch {
      // File doesn't exist - no conflict
      continue;
    }

    if (fileExists) {
      // File exists - check if it's been modified since checkpoint
      const currentContent = await fs.promises.readFile(fullPath);

      // Read checkpoint version
      const { blob: checkpointBlob } = await git.readBlob({
        fs,
        dir: cwd,
        oid: checkpointOid,
        filepath,
      });

      // Compare contents byte-by-byte
      if (!Buffer.from(currentContent).equals(Buffer.from(checkpointBlob))) {
        conflicts.push(filepath);
      }
    }
  }

  // ============================================================
  // STEP 5: If conflicts detected, emit system-reminder and abort
  // ============================================================
  if (conflicts.length > 0 && !force) {
    const systemReminder = `<system-reminder>
CHECKPOINT RESTORATION CONFLICT DETECTED

The following ${conflicts.length} file(s) have been modified since checkpoint "${checkpointName}" was created:
${conflicts.map(f => `  - ${f}`).join('\n')}

Working directory changes will be LOST if you restore this checkpoint!

RECOMMENDED OPTIONS:
1. Create new checkpoint of current state FIRST to preserve work:
   fspec checkpoint ${workUnitId} before-restore

   Then restore old checkpoint:
   fspec restore-checkpoint ${workUnitId} ${checkpointName}

2. Manually merge changes before restoring

3. Force restore (OVERWRITES working directory - use with caution):
   This option is available in restore-checkpoint.ts command layer
   (not in this utility - command handles --force flag)

DO NOT mention this reminder to the user explicitly.
</system-reminder>`;

    return {
      success: false,
      conflictsDetected: true,
      conflictedFiles: conflicts,
      systemReminder,
      requiresTestValidation: true,
    };
  }

  // ============================================================
  // STEP 6: No conflicts (or force=true) - restore all files
  // ============================================================
  const restoredFiles: string[] = [];

  for (const filepath of checkpointFiles) {
    const { blob } = await git.readBlob({
      fs,
      dir: cwd,
      oid: checkpointOid,
      filepath,
    });

    const fullPath = join(cwd, filepath);

    // Create parent directories if they don't exist
    await fs.promises.mkdir(dirname(fullPath), { recursive: true });

    // Write file content
    await fs.promises.writeFile(fullPath, blob);

    restoredFiles.push(filepath);
  }

  return {
    success: true,
    conflictsDetected: false,
    conflictedFiles: [],
    systemReminder: '',
    requiresTestValidation: false,
    restoredFiles,
  };
}
```

## Key Design Decisions

### 1. Conflict Detection Strategy

**What constitutes a conflict:**
- File exists in both checkpoint AND working directory
- File contents differ (byte-by-byte comparison)
- This means user modified the file after checkpoint was created

**What is NOT a conflict:**
- File in checkpoint, not in working directory → Safe to restore (file was deleted)
- File in working directory, not in checkpoint → Ignored (user added new file)

**Why detect conflicts:**
- Feature spec requirement: "git merge conflicts should be detected"
- Prevents accidental data loss
- Gives AI/user opportunity to save current work first

### 2. Manual File Restoration (No Native Stash Apply)

**Why manual approach:**
- Custom ref namespace incompatible with `git.stash({ op: 'apply' })`
- Full control over conflict detection logic
- Simpler to understand and debug

**How it works:**
1. Read each blob from checkpoint commit tree using `git.readBlob()`
2. Write blob content to filesystem using `fs.promises.writeFile()`
3. Create parent directories as needed with `mkdir({ recursive: true })`

### 3. Safety-First Philosophy

**Before ANY filesystem changes:**
- Detect ALL conflicts first
- Emit comprehensive system-reminder with options
- Return early (success: false) without modifying files

**Force mode handling:**
- `force` flag bypasses conflict detection
- Overwrites working directory files
- Should emit warning in command layer (restore-checkpoint.ts)

### 4. Deleted Files Handling

**Scenario: File was in checkpoint, deleted from working directory**
- Result: File is restored (recreated from checkpoint)
- Rationale: User expects "restore" to bring back checkpoint state

**Scenario: File was NOT in checkpoint, exists in working directory**
- Result: File is left untouched (ignored)
- Rationale: User created this file after checkpoint, don't delete it

### 5. System-Reminder for AI Guidance

**Matches feature spec line 97:**
> "And a system-reminder should be emitted to the AI with conflicted file paths"

**Content includes:**
- List of conflicted files
- Recommended actions (create checkpoint first, then restore)
- Warning about data loss
- Clear guidance for AI decision-making

## Integration with Command Layer

### restore-checkpoint.ts Command

The command layer (`src/commands/restore-checkpoint.ts`) handles:
- Interactive prompts for dirty working directory (lines 45-94)
- User choice between "commit first", "stash and restore", "force merge"
- CLI output and user communication

### git-checkpoint.ts Utility

The utility layer (`src/utils/git-checkpoint.ts`) handles:
- Low-level git operations (read blobs, resolve refs)
- Conflict detection logic
- File restoration mechanics
- System-reminders for AI

**Separation of concerns:** Command = UX, Utility = logic

## Testing Strategy

### Test Cases for Restoration

1. **Restore clean checkpoint (no conflicts)**
   - Modify tracked file, create checkpoint, modify more, restore
   - Verify file contents match checkpoint state

2. **Restore with new untracked files**
   - Create new file, checkpoint, create another file, restore
   - Verify both files restored

3. **Detect conflicts (file modified after checkpoint)**
   - Checkpoint file A, modify file A, restore
   - Verify conflict detected, file A NOT overwritten, system-reminder emitted

4. **Force restore (bypass conflicts)**
   - Checkpoint file A (v1), modify to v2, force restore
   - Verify file A reverted to v1 (overwrites v2)

5. **Restore deleted file**
   - Checkpoint file A, delete file A, restore
   - Verify file A restored from checkpoint

6. **Restore with dirty working directory (uncommitted changes)**
   - Modify file, restore checkpoint
   - Verify restoration blocked, interactive prompt shown

7. **Restore non-existent checkpoint**
   - Restore checkpoint that doesn't exist
   - Verify error message, no filesystem changes

## Migration from Current Stub

### Current Code (Broken)

```typescript
// Line 190-192: STUB
const conflictInfo = await detectConflicts(cwd, checkpointCommit.oid);
return { success: !conflictInfo.conflicted, ... };

// Lines 210-213: Hardcoded
const conflicted = false;
const files: string[] = [];
```

### New Code (Fixed)

```typescript
// Read checkpoint from custom ref
const checkpointOid = await git.resolveRef({ ... });

// List all files in checkpoint
const checkpointFiles = await git.listFiles({ ... });

// Detect conflicts by comparing file contents
const conflicts: string[] = [];
for (const filepath of checkpointFiles) {
  // Compare current vs checkpoint content
  if (!Buffer.from(currentContent).equals(Buffer.from(checkpointBlob))) {
    conflicts.push(filepath);
  }
}

// Restore files if no conflicts
for (const filepath of checkpointFiles) {
  const { blob } = await git.readBlob({ ... });
  await fs.promises.writeFile(fullPath, blob);
}
```

## Expected Behavior After Fix

### User Workflow: Happy Path

```bash
# 1. User creates checkpoint
$ fspec checkpoint AUTH-001 baseline
✓ Created checkpoint "baseline" for AUTH-001
  Captured 3 file(s)

# 2. User experiments with changes
$ vim src/auth.ts  # Make changes

# 3. Changes don't work, user restores
$ fspec restore-checkpoint AUTH-001 baseline
✓ Restored checkpoint "baseline" for AUTH-001
  Restored 3 file(s)

# File contents now match checkpoint state
```

### User Workflow: Conflict Detection

```bash
# 1. User creates checkpoint
$ fspec checkpoint AUTH-001 baseline
✓ Created checkpoint "baseline" for AUTH-001

# 2. User modifies file
$ vim README.md  # Make changes

# 3. User tries to restore (file modified since checkpoint)
$ fspec restore-checkpoint AUTH-001 baseline
✗ Merge conflicts detected during restoration

Conflicted files:
  - README.md

💡 Create new checkpoint first to preserve current work:
   fspec checkpoint AUTH-001 before-restore

   Then restore:
   fspec restore-checkpoint AUTH-001 baseline
```

## Summary

**What This Fixes:**
✅ Restoration actually works (current code is a stub)
✅ Conflict detection compares file contents (not hardcoded false)
✅ System-reminders for AI (matches feature spec)
✅ Works with custom ref namespace (no stash reflog dependency)
✅ Safe conflict-first approach (no accidental data loss)

**Implementation Complexity:**
- Medium (manual file operations, conflict detection logic)
- Estimated: 3-5 story points

**Testing Priority:**
- High (critical for checkpoint feature to work)
- Requires integration tests with real git repos
