# GIT-035 Investigation Findings

## Session Under Investigation

- **Session ID**: `e79d413f-fa04-4be5-b6d8-ab052217c129`
- **Worktree Path**: `.fspec/worktrees/e79d413f-fa04-4be5-b6d8-ab052217c129/`
- **Base Commit**: `88c4d7b91ed4cb077bf10378aaf2856895e623dc`

## Screenshot Analysis

The Session Management Panel shows:
```
Session Management
Manage isolated session worktrees

▶ [active] e79d413f... (0 files changed)

Changes:
Modified: none
Added: none
Deleted: none

↑↓ Navigate | M Merge | D Discard | R Refresh | Esc Close
```

**UI Claims**: 0 files changed, no modifications, no additions, no deletions.

## Actual Git State

```bash
$ cd .fspec/worktrees/e79d413f-fa04-4be5-b6d8-ab052217c129

$ git ls-files | wc -l
0
# The git index is EMPTY

$ git diff --cached --stat | tail -5
 test-fixtures/script.sh                            |    19 -
 tsconfig.json                                      |    22 -
 vite.config.ts                                     |    86 -
 vitest.config.ts                                   |    31 -
 3395 files changed, 1060613 deletions(-)

$ git status
Not currently on any branch.
Changes to be committed:
  (use "git restore --staged <file>..." to unstage)
        deleted:    .gitignore
        deleted:    .husky/pre-commit
        deleted:    .npmignore
        ... (3395 files total)

$ ls -la | head -10
total 1788
drwxrwxr-x 11 rquast rquast   4096 Feb 22 13:59 .
drwxrwxr-x  3 rquast rquast   4096 Feb 22 13:59 ..
-rw-rw-r--  1 rquast rquast  11781 Feb 22 13:59 AGENTS.md
-rw-rw-r--  1 rquast rquast 248624 Feb 22 13:59 attachment1.png
# Files ARE present in working directory
```

## Root Cause Analysis

### Problem 1: Empty Git Index

The worktree was created with an **empty git index**. The working directory has all files correctly, but `git ls-files` returns 0 files. This causes git to interpret all files as "staged for deletion".

**Location**: `codelet/git/src/worktree.rs` or `codelet/git/src/isolated_session.rs`

The `create_worktree()` function likely:
1. Creates the worktree directory ✓
2. Copies/checks out files to working directory ✓
3. **Does NOT initialize the git index from HEAD** ✗

### Problem 2: get_session_diff() Doesn't Detect This

In `codelet/git/src/session_result.rs`:

```rust
pub fn get_session_diff(repo_path: impl AsRef<Path>, session_id: &str) -> Result<SessionResult> {
    // Get the base commit tree
    let base_tree_files = get_tree_files(&repo, &base_commit)?;
    
    // Get current worktree files
    let worktree_files = collect_worktree_files(&worktree_path)?;
    
    // Compare base_tree vs worktree (BOTH have same files!)
    // This misses the empty index problem entirely
}
```

The function compares **working directory** to **base commit tree**, not to the **git index**. Since both have the same files, it reports "0 changes".

### Problem 3: Session Management Panel Keyboard Not Bound

The Session Management Panel shows keybindings:
- `↑↓ Navigate` - Not working
- `M Merge` - Not working  
- `D Discard` - Not working
- `R Refresh` - Not working
- `Esc Close` - Not working

The `useInputCompat` hook in `SessionManagementPanel.tsx` is registered but inputs are not being processed. Likely cause:
1. `isActive` prop not being passed correctly
2. Input priority conflict with parent component
3. Dialog not properly capturing focus

## Impact Assessment

| Component | Impact |
|-----------|--------|
| Session Management Panel | Shows incorrect "0 files changed" |
| Merge operation | Would fail or produce wrong results |
| Discard operation | May not clean up correctly |
| AI agents in isolated sessions | Git commands return unexpected results |
| File search (@file popup) | Related to GIT-033 - searches wrong path |

## Files to Fix

1. **`codelet/git/src/worktree.rs`** or **`codelet/git/src/isolated_session.rs`**
   - Initialize git index from HEAD after worktree creation
   - Use `git reset --mixed HEAD` or equivalent gix operation

2. **`codelet/git/src/session_result.rs`**
   - `get_session_diff()` should also check index state
   - Detect corrupted index and report error or fix it

3. **`src/tui/components/SessionManagementPanel.tsx`**
   - Debug why keyboard input is not being processed
   - Check `isActive` prop and input priority

## Reproduction Steps

1. Create an isolated session from the TUI
2. Open Session Management Panel (how? keybinding not documented)
3. Observe "0 files changed" even though worktree is corrupted
4. Try to use M/D/R keys - nothing happens
5. Check worktree with `git status` - see all files as deleted

## Related Work Units

- **GIT-019** (Isolated Session Creation) - marked done but has this bug
- **GIT-029** (TUI integration for isolated sessions) - marked done but incomplete
- **GIT-033** (File Search Uses Worktree Path) - in specifying, related issue

## Recommended Fix Order

1. Fix worktree creation to initialize index properly
2. Add validation in `get_session_diff()` to detect corrupted state
3. Fix keyboard input in SessionManagementPanel
4. Add test for worktree index initialization
5. Clean up orphaned corrupted worktree `e79d413f...`
