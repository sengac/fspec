# AST Research: Worktree Index Initialization Fix

## Overview

This research documents the AST-based code analysis for GIT-035 - fixing the empty git index bug in worktree creation.

## AST Pattern Searches

### 1. Function definitions in worktree.rs

**Pattern:** `fn $NAME($$$ARGS) -> Result<$RET> { $$$BODY }`

**Results:**
- `resolve_commit_ref` - Resolves commit refs using gitoxide
- `initialize_worktree_index` - **NEW** - Initializes index from tree using pure gitoxide
- `write_worktree_metadata` - Writes git worktree metadata files

### 2. Key gitoxide API calls identified

**Pattern:** `repo.$METHOD($$$ARGS)`

Key operations:
- `repo.index_from_tree(&tree_id)` - Creates index state from tree (gix API)
- `repo.find_object(*commit_id)` - Gets commit object (gix API)
- `index_file.write_to(&mut writer, options)` - Writes index to disk (gix API)

## Implementation Details

### The Fix

Added `initialize_worktree_index()` function that:

1. **Gets the tree from commit** using `repo.find_object().into_commit().tree_id()`
2. **Creates index from tree** using `repo.index_from_tree(&tree_id)` 
3. **Sets index path** to `worktree_git_dir/index`
4. **Writes index to disk** using `index_file.write_to()`

### CRITICAL: Pure Gitoxide Implementation

**NO git CLI commands are used.** All operations use gitoxide (gix) APIs:

- ❌ NOT: `std::process::Command::new("git")`
- ✅ YES: `gix::Repository`, `gix::index::File`, etc.

### Files Modified

1. `codelet/git/src/worktree.rs`:
   - Added `initialize_worktree_index()` function
   - Updated module documentation to emphasize PURE GITOXIDE
   - Added call to `initialize_worktree_index()` after checkout

2. `codelet/git/src/checkout.rs`:
   - Updated module documentation to emphasize PURE GITOXIDE

## Test Verification

All 4 tests in `worktree_index_initialization_tests.rs` now pass:
- `test_worktree_has_all_tracked_files_in_git_index` ✅
- `test_worktree_has_clean_git_status_after_creation` ✅
- `test_session_diff_shows_accurate_file_change_count` ✅
- `test_session_diff_detects_corrupted_empty_index` ✅
