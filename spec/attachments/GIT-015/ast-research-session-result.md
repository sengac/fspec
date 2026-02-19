# AST Research: Session Result Collection and Patch Application

## Summary

This document contains AST analysis of the codelet-git module to understand what exists 
and what needs to be implemented for GIT-015.

## Existing Public Functions (codelet/git/src)

### worktree.rs
| Function | Signature | Purpose |
|----------|-----------|---------|
| `create_worktree` | `pub fn create_worktree(repo_path, session_id) -> Result<WorktreeCreateResult>` | Create worktree at HEAD |
| `create_worktree_at_ref` | `pub fn create_worktree_at_ref(repo_path, session_id, commit_ref) -> Result<WorktreeCreateResult>` | Create worktree at specific commit |
| `remove_worktree` | `pub fn remove_worktree(repo_path, session_id) -> Result<()>` | Remove worktree and cleanup |
| `list_worktrees` | `pub fn list_worktrees(repo_path) -> Result<Vec<WorktreeInfo>>` | List all session worktrees |

### diff.rs
| Function | Signature | Purpose |
|----------|-----------|---------|
| `get_file_diff` | `pub fn get_file_diff(dir, filepath) -> Result<Option<String>>` | Get unified diff for file vs HEAD |
| `is_binary_file` | `pub fn is_binary_file(dir, filepath) -> Result<bool>` | Check if file is binary |

### status.rs
| Function | Signature | Purpose |
|----------|-----------|---------|
| `get_staged_files` | `pub fn get_staged_files(dir) -> Result<Vec<String>>` | List staged files |
| `get_unstaged_files` | `pub fn get_unstaged_files(dir) -> Result<Vec<String>>` | List unstaged modified files |
| `get_untracked_files` | `pub fn get_untracked_files(dir) -> Result<Vec<String>>` | List untracked files |
| `get_current_branch` | `pub fn get_current_branch(dir) -> Result<Option<String>>` | Get current branch name |

## Existing Structs

### worktree.rs
```rust
pub struct WorktreeInfo {
    pub session_id: String,
    pub path: PathBuf,
    pub head_commit: String,
    pub is_detached: bool,
}

pub struct WorktreeCreateResult {
    pub info: WorktreeInfo,
    pub base_commit: String,
    pub created_at: DateTime<Utc>,
}
```

## What Needs to Be Implemented

### New Struct: SessionResult
```rust
pub struct SessionResult {
    pub session_id: String,
    pub diff: String,           // Unified diff format
    pub files_changed: Vec<String>,
    pub files_added: Vec<String>,
    pub files_deleted: Vec<String>,
    pub base_commit: String,
}
```

### New Error Variant
```rust
// In error.rs
ConflictError {
    files: Vec<String>,
    message: String,
}
```

### New Functions in worktree.rs

1. **get_session_diff(repo_path, session_id) -> Result<SessionResult>**
   - Compare base_commit tree against current worktree WORKING DIRECTORY
   - Return SessionResult with diff and file lists
   - No side effects - worktree remains intact

2. **apply_session_changes(repo_path, session_id) -> Result<()>**
   - Copy modified files from session worktree to main worktree
   - Copy added files from session worktree to main worktree
   - Delete files that were deleted in session
   - Detect conflicts (files modified in both)
   - Remove worktree after successful apply

3. **abort_session(repo_path, session_id) -> Result<()>**
   - Simply calls remove_worktree (alias for clarity)

### NAPI Bindings Needed (codelet/napi/src/git.rs)

From GIT-014 (not yet exposed):
- `createWorktree(dir, sessionId) -> WorktreeCreateResult`
- `createWorktreeAtRef(dir, sessionId, commitRef) -> WorktreeCreateResult`
- `removeWorktree(dir, sessionId) -> void`
- `listWorktrees(dir) -> WorktreeInfo[]`

From GIT-015:
- `getSessionDiff(dir, sessionId) -> SessionResult`
- `applySessionChanges(dir, sessionId) -> void`
- `abortSession(dir, sessionId) -> void`

## Implementation Notes

1. **Diff generation** - Reuse existing diff.rs infrastructure
   - Compare base_commit tree to worktree working directory
   - Walk both trees and compare files
   
2. **File copying approach** (Rule [9])
   - NOT using git apply/patch operations
   - Direct file copy from session worktree to main worktree
   - Simpler and more predictable

3. **Conflict detection** (Rule [6])
   - Before applying, check if files in main worktree have changed since base_commit
   - If yes, return ConflictError with list of conflicting files
   - Session worktree NOT removed on conflict

4. **Binary files** 
   - Diff shows "[Binary file]" indicator
   - Apply still copies the binary file correctly
