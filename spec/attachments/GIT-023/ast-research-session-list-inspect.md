# AST Research: Session Manager List and Inspect

## Overview

This document summarizes the AST research performed for GIT-023, analyzing the existing session management infrastructure that this story builds upon.

## Existing Infrastructure (from GIT-022)

### session_status.rs

**DerivedSessionStatus enum** (lines 24-34):
```rust
pub enum DerivedSessionStatus {
    Active,      // Session is currently active
    PendingMerge, // Has uncommitted changes, ready for merge
    Clean,       // No uncommitted changes
    Orphaned,    // No valid session record (manifest missing/terminated)
}
```

**derive_session_status function** (lines 183-236):
- Takes `repo_path`, `session_id`, and `active_sessions: &HashSet<String>`
- Priority order:
  1. Active sessions map → Active
  2. Worktree not found → WorktreeNotFound error
  3. Manifest missing/terminated → Orphaned
  4. Worktree has changes → PendingMerge
  5. Otherwise → Clean

**SessionManifest struct** (lines 53-70):
- `session_id: String`
- `project_root: PathBuf`
- `worktree_path: Option<PathBuf>`
- `base_commit: Option<String>`
- `created_at: DateTime<Utc>`
- `completed_at: Option<DateTime<Utc>>`
- `terminated: bool`

### worktree.rs

**WorktreeInfo struct** (lines 17-27):
```rust
pub struct WorktreeInfo {
    pub session_id: String,
    pub path: PathBuf,
    pub head_commit: String,
    pub is_detached: bool,
}
```

**list_worktrees function** (lines 148-192):
- Scans `.fspec/worktrees/` directory
- Returns `Vec<WorktreeInfo>` for all worktrees found
- Returns empty Vec if no worktrees exist

### session_result.rs

**SessionResult struct** (lines 18-32):
```rust
pub struct SessionResult {
    pub session_id: String,
    pub diff: String,
    pub files_changed: Vec<String>,
    pub files_added: Vec<String>,
    pub files_deleted: Vec<String>,
    pub base_commit: String,
}
```

**get_session_diff function** (lines 45-115):
- Takes `repo_path` and `session_id`
- Returns `SessionResult` with unified diff and file lists
- Returns `WorktreeNotFound` error if session doesn't exist

## Implementation Plan for GIT-023

### New Types to Add

**SessionInfo struct** (combine WorktreeInfo with status):
```rust
pub struct SessionInfo {
    pub session_id: String,
    pub status: DerivedSessionStatus,
    pub base_commit: String,
    pub files_changed: usize,
    pub created_at: DateTime<Utc>,
    pub worktree_path: PathBuf,
}
```

**SessionFilter enum**:
```rust
pub enum SessionFilter {
    All,
    Active,
    PendingMerge,
    Clean,
    Orphaned,
}
```

### New Functions to Implement

**list_sessions function**:
```rust
pub fn list_sessions(
    repo_path: &Path,
    active_sessions: &HashSet<String>,
    filter: SessionFilter,
) -> Result<Vec<SessionInfo>>
```

Implementation approach:
1. Call `list_worktrees(repo_path)`
2. For each worktree, call `derive_session_status()`
3. Apply filter
4. Call `get_session_diff()` to get file count
5. Read manifest for `created_at`
6. Return `Vec<SessionInfo>`

**inspect_session function** (thin wrapper):
```rust
pub fn inspect_session(
    repo_path: &Path,
    session_id: &str,
) -> Result<SessionResult>
```

Implementation: Simply delegate to `get_session_diff()`.

### File Location

Add to existing `codelet/git/src/session_status.rs` alongside:
- `DerivedSessionStatus` enum
- `derive_session_status()` function

## Integration Points

- Uses `list_worktrees()` from `worktree.rs` (GIT-014)
- Uses `derive_session_status()` from `session_status.rs` (GIT-022)
- Uses `get_session_diff()` from `session_result.rs` (GIT-015)
- Uses `read_manifest()` from `session_status.rs` (GIT-022)

## Test Strategy

Tests should be placed in `codelet/git/tests/session_list_inspect_test.rs`:

1. Create test repo with multiple worktrees
2. Set up different status conditions (active, pending_merge, clean, orphaned)
3. Test list_sessions with each filter type
4. Test inspect_session returns correct diff
5. Test empty result when no worktrees
6. Test error handling for non-existent session

## Dependencies

- **GIT-014**: list_worktrees() ✓ (done)
- **GIT-015**: get_session_diff() ✓ (done)
- **GIT-022**: derive_session_status() ✓ (done)
