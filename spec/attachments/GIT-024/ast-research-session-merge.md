# AST Research: Session Merge Operations (GIT-024)

## Overview

This document captures AST analysis of the existing session management code to inform the implementation of `merge_session()`.

## session_result.rs - Existing Primitives

### Public Functions

| Function | Line | Signature |
|----------|------|-----------|
| `get_session_diff` | 45 | `pub fn get_session_diff(repo_path: impl AsRef<Path>, session_id: &str) -> Result<SessionResult>` |
| `apply_session_changes` | 128 | `pub fn apply_session_changes(repo_path: impl AsRef<Path>, session_id: &str) -> Result<()>` |
| `abort_session` | 204 | `pub fn abort_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<()>` |

### Key Insight

The `apply_session_changes()` function already:
1. Gets diff from `get_session_diff()`
2. Detects conflicts between session and main worktree
3. Copies modified/added files to main
4. Deletes removed files from main
5. Removes session worktree on success

**GIT-024 wraps this with a return struct** - `MergeResult` captures what changed.

## session_status.rs - GIT-022/GIT-023 Implementation

### Public Functions

| Function | Line | Purpose |
|----------|------|---------|
| `get_sessions_dir` | 108 | Returns `~/.fspec/git-sessions/` path |
| `get_manifest_path` | 113 | Returns manifest file path for session |
| `read_manifest` | 118 | Reads session manifest from disk |
| `write_manifest` | 135 | Writes session manifest to disk |
| `delete_manifest` | 155 | Deletes session manifest |
| `derive_session_status` | 183 | Derives status (Active/PendingMerge/Clean/Orphaned) |
| `complete_session` | 250 | Marks session completed in manifest |
| `create_session_manifest` | 274 | Creates new session manifest |
| `terminate_session` | 297 | Marks session as terminated (orphaned) |
| `list_sessions` | 364 | Lists all sessions with derived status |
| `inspect_session` | 434 | Inspects session diff (read-only wrapper) |

### Implementation Location

**GIT-024 should add `merge_session()` at line ~440+** after `inspect_session()`.

## Implementation Plan

### MergeResult Struct

```rust
/// Result of merging a session to main worktree
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Session ID that was merged
    pub session_id: String,
    /// Files that were modified in main
    pub files_modified: Vec<String>,
    /// Files that were added to main  
    pub files_added: Vec<String>,
    /// Files that were deleted from main
    pub files_deleted: Vec<String>,
}
```

### merge_session() Function

```rust
/// Merge session changes to main worktree
///
/// Applies all changes from session to main and removes worktree on success.
/// Returns conflict error if main has diverged since session base commit.
///
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
///
/// # Returns
/// MergeResult on success, or error if conflicts detected
pub fn merge_session(
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> Result<MergeResult> {
    // 1. Get diff first (to capture what will change)
    let diff = get_session_diff(repo_path.as_ref(), session_id)?;
    
    // 2. Apply changes (this handles conflicts and cleanup)
    apply_session_changes(repo_path, session_id)?;
    
    // 3. Delete manifest (cleanup)
    delete_manifest(session_id)?;
    
    // 4. Return what was merged
    Ok(MergeResult {
        session_id: session_id.to_string(),
        files_modified: diff.files_changed,
        files_added: diff.files_added,
        files_deleted: diff.files_deleted,
    })
}
```

## Dependencies

- `get_session_diff()` from session_result.rs (GIT-015)
- `apply_session_changes()` from session_result.rs (GIT-015)
- `delete_manifest()` from session_status.rs (GIT-022)

## Test Strategy

Tests should verify:
1. Simple merge with modified files → correct MergeResult
2. Merge with added files → files_added populated
3. Merge with deleted files → files_deleted populated
4. Conflict detection → ConflictError returned
5. Worktree cleanup on success
6. Worktree intact on conflict
7. Clean session merge (no changes) → empty MergeResult, worktree removed
