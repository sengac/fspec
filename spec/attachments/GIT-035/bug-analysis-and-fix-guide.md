# GIT-035: Worktree Empty Index Bug - Analysis and Fix Guide

## Summary

When gitoxide creates a worktree for isolated sessions, the git index is left **empty**. This causes all tracked files to appear as "staged for deletion" in `git status`, even though the files are physically present in the worktree directory.

## Symptoms

### Current (Buggy) Behavior

```bash
# Create isolated session
$ fspec session create --isolated

# In the worktree directory:
$ git status
D  README.md
D  src/config.rs  
D  src/main.rs
D  src/old.rs
?? README.md
?? src/

$ git ls-files
# (returns nothing - empty!)

$ git ls-files | wc -l
0
```

### Expected Behavior (After Fix)

```bash
# In the worktree directory:
$ git status
nothing to commit, working tree clean

$ git ls-files
README.md
src/config.rs
src/main.rs
src/old.rs

$ git ls-files | wc -l
4
```

## Root Cause

The gitoxide worktree creation code copies the working directory files but does NOT initialize the git index from HEAD. The index file is either:
1. Not created at all, or
2. Created but empty

This means git thinks all files have been staged for deletion (they're in HEAD but not in the index).

## Impact

1. **Session Management Panel** shows incorrect file counts (likely all files as "deleted")
2. **`get_session_diff()`** returns inaccurate diff information
3. **Any git operation** in the worktree is confused about file state

## Location of Bug

The bug is in the worktree creation code:

- **Primary suspect**: `codelet/git/src/worktree.rs`
- **Alternative location**: `codelet/git/src/isolated_session.rs`

Look for the function that calls gitoxide to create the worktree. After that call completes, the index needs to be populated.

## The Fix

After worktree creation, run the gitoxide equivalent of:

```bash
git reset --mixed HEAD
```

This command:
1. Keeps the working directory files unchanged
2. Resets the index to match HEAD
3. Results in a clean `git status`

### Gitoxide API Hint

In gitoxide (gix), you likely need to:

```rust
// After worktree is created, open it as a repository
let worktree_repo = gix::open(worktree_path)?;

// Read HEAD tree
let head_commit = worktree_repo.head_commit()?;
let head_tree = head_commit.tree()?;

// Reset index to match HEAD tree
// Look for: worktree_repo.index_mut() or similar
// Then iterate head_tree entries and add to index
```

Research the gix API for:
- `gix::Repository::index()` or `index_mut()`
- Writing/updating the index
- Reading tree entries from a commit

## Tests Already Written

The tests are in `codelet/git/tests/worktree_index_initialization_tests.rs`:

| Test | Description | Currently |
|------|-------------|-----------|
| `test_worktree_has_all_tracked_files_in_git_index` | `git ls-files` returns all files | ❌ FAILING |
| `test_worktree_has_clean_git_status_after_creation` | `git status --porcelain` is empty | ❌ FAILING |
| `test_session_diff_shows_accurate_file_change_count` | Modify 1 file → panel shows 1 changed | ✅ PASSING |
| `test_session_diff_detects_corrupted_empty_index` | Detects/handles corrupted index | ✅ PASSING |

The first two tests are failing because they expose the bug. After the fix, they should pass.

## Verification Steps

After implementing the fix:

```bash
# Run the specific tests
cargo test -p codelet-git --test worktree_index_initialization_tests

# All 4 tests should pass
```

## DO NOT

- ❌ Shell out to `git` CLI - use gitoxide (gix) only
- ❌ Modify the test expectations to make them pass
- ❌ Skip the index initialization "for now"

## Related Files

- `codelet/git/src/worktree.rs` - Worktree creation
- `codelet/git/src/isolated_session.rs` - Isolated session creation  
- `codelet/git/tests/worktree_index_initialization_tests.rs` - Tests (already written)
- `codelet/git/tests/common/mod.rs` - Test helpers
