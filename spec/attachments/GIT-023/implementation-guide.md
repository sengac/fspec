# GIT-023: Session Manager List and Inspect

## Overview

This story implements the session listing and inspection operations. Users can list all session worktrees with their derived status and inspect a session's diff without any side effects.

## Problem Statement

Users need to see what sessions exist and their status (active, pending_merge, clean, orphaned). They also need to preview changes before deciding to merge or discard.

## Solution

1. Implement `list_sessions(filter?)` that returns all sessions with derived status
2. Implement `inspect_session(id)` that returns diff without modifying anything
3. Support filtering by status (e.g., only orphaned, only pending_merge)

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| List all session worktrees with status information | Returns all sessions with computed status |
| List only orphaned session worktrees | Filter by status=orphaned |
| List sessions with pending_merge filter | Filter by status=pending_merge |
| List sessions returns empty when no worktrees exist | Empty list if no worktrees |
| Inspect session diff before merging | Returns diff without side effects |
| Inspect session shows deleted files | Deleted files included in diff |
| Inspect clean session returns empty diff | No changes = empty diff |
| Inspect session fails for non-existent session | WorktreeNotFound error |

## Implementation Location

### Create New Module

```
codelet/git/src/session_manager.rs (NEW)
├── SessionInfo struct
├── SessionFilter enum
├── list_sessions(repo_path, active_sessions, filter?) -> Vec<SessionInfo>
└── inspect_session(repo_path, session_id) -> SessionResult
```

### Integration with Existing Modules

```
codelet/git/src/worktree.rs (GIT-014)
└── list_worktrees(repo_path) -> Vec<WorktreeInfo>

codelet/git/src/session_result.rs (GIT-015)  
└── get_session_diff(repo_path, session_id) -> SessionResult
```

## API Design

### SessionInfo Struct

```rust
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session ID
    pub session_id: String,
    /// Derived status
    pub status: SessionStatus,
    /// Base commit the worktree was created from
    pub base_commit: String,
    /// Number of files changed (modified + added + deleted)
    pub files_changed: usize,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// Path to the worktree
    pub worktree_path: PathBuf,
}
```

### Filter Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionFilter {
    /// Return all sessions
    All,
    /// Only active sessions
    Active,
    /// Only pending_merge sessions
    PendingMerge,
    /// Only clean sessions
    Clean,
    /// Only orphaned sessions
    Orphaned,
}
```

### List Sessions Function

```rust
pub fn list_sessions(
    repo_path: &Path,
    active_sessions: &HashSet<String>,
    filter: SessionFilter,
) -> Result<Vec<SessionInfo>> {
    let worktrees = list_worktrees(repo_path)?;
    
    let mut sessions = Vec::new();
    for worktree in worktrees {
        let status = derive_session_status(
            repo_path, 
            &worktree.session_id, 
            active_sessions
        )?;
        
        // Apply filter
        if !matches_filter(&status, &filter) {
            continue;
        }
        
        // Get change count
        let diff = get_session_diff(repo_path, &worktree.session_id)?;
        let files_changed = diff.files_changed.len() 
            + diff.files_added.len() 
            + diff.files_deleted.len();
        
        sessions.push(SessionInfo {
            session_id: worktree.session_id,
            status,
            base_commit: worktree.head_commit,
            files_changed,
            created_at: read_manifest_created_at(&worktree.session_id)?,
            worktree_path: worktree.path,
        });
    }
    
    Ok(sessions)
}
```

### Inspect Session Function

```rust
/// Inspect session diff without any side effects
/// 
/// This is essentially a wrapper around get_session_diff that ensures
/// the worktree remains intact after inspection.
pub fn inspect_session(
    repo_path: &Path,
    session_id: &str,
) -> Result<SessionResult> {
    // get_session_diff already doesn't modify anything
    get_session_diff(repo_path, session_id)
}
```

## Test Strategy

Tests in `codelet/git/tests/session_manager_list_inspect_test.rs`:

1. **List all sessions**: Multiple sessions with different statuses
2. **Filter by status**: Verify filtering works correctly
3. **Empty list**: No worktrees returns empty list
4. **Inspect with changes**: Returns correct diff
5. **Inspect deleted files**: Deleted files in diff
6. **Inspect clean session**: Empty diff for no changes
7. **Inspect non-existent**: WorktreeNotFound error

## Dependencies

- **GIT-022** (required): Provides status derivation logic

## Downstream Dependencies

- **GIT-024**: Merge operations need to list/inspect first
- **GIT-025**: Discard operations may list first
- **GIT-026**: Orphan pruning needs to list orphaned sessions
- **GIT-027**: NAPI bindings expose these operations

## Acceptance Criteria Checklist

- [ ] `SessionInfo` struct with all required fields
- [ ] `SessionFilter` enum for filtering
- [ ] `list_sessions()` returns all sessions with status
- [ ] Filtering by status works correctly
- [ ] Empty list returned when no worktrees exist
- [ ] `inspect_session()` returns diff without side effects
- [ ] Deleted files shown in inspection diff
- [ ] Clean sessions show empty diff
- [ ] WorktreeNotFound error for non-existent sessions
- [ ] All tests pass

---

## Next Steps

GIT-023 **unlocks three parallel stories**. Once complete:

| Story | Title | Why It's Next |
|-------|-------|---------------|
| **GIT-024** | Session Manager Merge Operations | Uses inspect before merging |
| **GIT-025** | Session Manager Discard Operations | May list sessions before discarding |
| **GIT-026** | Orphan Detection and Pruning | Lists orphaned sessions before pruning |

All three can be worked on in parallel after GIT-023 is complete.

## Story Dependency Graph

```
GIT-022 (Status Derivation)
    │
    └── GIT-023 (This Story) ◀── LIST/INSPECT - CENTRAL HUB
            │
            ├── GIT-024 (Merge) ◀── PARALLEL
            │
            ├── GIT-025 (Discard) ◀── PARALLEL
            │
            └── GIT-026 (Orphan Pruning) ◀── PARALLEL
                    │
                    └── GIT-027 (NAPI Bindings) ◀── WAITS FOR ALL
```

## Related Stories

| Story | Relationship | Notes |
|-------|--------------|-------|
| GIT-022 | **Depends On** | Provides status derivation logic |
| GIT-024 | **Unlocks** | Merge operations need list/inspect |
| GIT-025 | **Unlocks** | Discard operations may list first |
| GIT-026 | **Unlocks** | Orphan pruning needs orphaned filter |
| GIT-027 | **Required By** | NAPI bindings expose list/inspect operations |
