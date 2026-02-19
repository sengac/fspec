# GIT-025: Session Manager Discard Operations

## Overview

This story implements the discard operation that removes a session worktree without applying any changes. This uses the `abort_session()` primitive from GIT-015.

## Problem Statement

Users may decide not to use a session's changes. They need a way to discard the session and clean up the worktree without affecting the main project.

## Solution

1. Implement `discard_session(id)` that removes worktree without applying changes
2. Use `abort_session()` primitive from GIT-015
3. Works for any session status (pending_merge, clean, orphaned)

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| Discard session without applying changes | Worktree removed, main unchanged |
| Discard clean session without confirmation | No special handling needed |
| Discard session fails for non-existent session | WorktreeNotFound error |
| Discard orphaned session removes worktree | Orphaned sessions can be discarded |

## Implementation Location

### Add to Session Manager Module

```
codelet/git/src/session_manager.rs
├── discard_session(repo_path, session_id) -> Result<DiscardResult>
└── DiscardResult struct
```

### Uses GIT-015 Primitives

```
codelet/git/src/session_result.rs (GIT-015)
└── abort_session(repo_path, session_id) -> Result<()>
```

## API Design

### DiscardResult Struct

```rust
#[derive(Debug, Clone)]
pub struct DiscardResult {
    /// Session ID that was discarded
    pub session_id: String,
    /// Number of files that were in the session (not applied)
    pub files_discarded: usize,
    /// Status the session had before discard
    pub previous_status: SessionStatus,
}
```

### Discard Function

```rust
/// Discard session without applying any changes
/// 
/// This removes the worktree and cleans up git metadata without
/// applying any of the session's changes to the main worktree.
/// 
/// # Arguments
/// * `repo_path` - Path to the main git repository
/// * `session_id` - Session identifier
/// 
/// # Errors
/// - WorktreeNotFound if session doesn't exist
pub fn discard_session(
    repo_path: &Path,
    session_id: &str,
) -> Result<DiscardResult> {
    // Get info before discarding
    let diff = get_session_diff(repo_path, session_id)?;
    let files_discarded = diff.files_changed.len() 
        + diff.files_added.len() 
        + diff.files_deleted.len();
    
    // Determine status before discard (for informational purposes)
    let previous_status = derive_session_status(
        repo_path, 
        session_id, 
        &HashSet::new() // Not active if we're discarding
    )?;
    
    // Remove worktree (abort_session is alias for remove_worktree)
    abort_session(repo_path, session_id)?;
    
    // Clean up session manifest
    remove_session_manifest(session_id)?;
    
    Ok(DiscardResult {
        session_id: session_id.to_string(),
        files_discarded,
        previous_status,
    })
}
```

### Usage Example

```rust
// Discard a session after reviewing its changes
let sessions = list_sessions(repo_path, &active, SessionFilter::PendingMerge)?;

for session in sessions {
    let diff = inspect_session(repo_path, &session.session_id)?;
    println!("Session {} has {} changes", session.session_id, diff.files_changed.len());
    
    // User decides to discard
    if user_confirms_discard() {
        let result = discard_session(repo_path, &session.session_id)?;
        println!("Discarded {} files", result.files_discarded);
    }
}
```

## Manifest Cleanup

When discarding, also remove the session manifest:

```rust
fn remove_session_manifest(session_id: &str) -> Result<()> {
    let manifest_path = dirs::home_dir()
        .ok_or(SessionError::NoHomeDir)?
        .join(".fspec")
        .join("sessions")
        .join(format!("{}.json", session_id));
    
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)?;
    }
    
    Ok(())
}
```

## Test Strategy

Tests in `codelet/git/tests/session_manager_discard_test.rs`:

1. **Discard with changes**: Worktree removed, main unchanged
2. **Discard clean session**: Works the same as with changes
3. **Discard non-existent**: WorktreeNotFound error
4. **Discard orphaned**: Orphaned sessions can be discarded
5. **Manifest cleanup**: Session manifest removed after discard

## Dependencies

- **GIT-023** (required): List/inspect may be used before discard
- **GIT-015** (done): Provides `abort_session()` primitive

## Downstream Dependencies

- **GIT-027**: NAPI bindings expose discard operation

## Acceptance Criteria Checklist

- [ ] `discard_session()` removes worktree
- [ ] Main worktree unchanged after discard
- [ ] Session manifest removed after discard
- [ ] WorktreeNotFound error for non-existent sessions
- [ ] Orphaned sessions can be discarded
- [ ] DiscardResult includes files_discarded count
- [ ] All tests pass

---

## Next Steps

GIT-025 is a **parallel story** with GIT-024 and GIT-026. Once complete:

| Action | Description |
|--------|-------------|
| **Mark Done** | Move GIT-025 to `done` status |
| **Verify Integration** | Discard operations work correctly |
| **Check GIT-027** | If GIT-024 and GIT-026 are also done, GIT-027 can start |

## Story Dependency Graph

```
GIT-023 (List/Inspect)
    │
    ├── GIT-024 (Merge)      │
    │           │            │
    │           └────────────┤
    │                        │
    ├── GIT-025 (This Story) ◀── DISCARD OPERATIONS
    │           │            │
    │           └────────────┤
    │                        │
    └── GIT-026 (Orphans)    │
                │            │
                └────────────┤
                             ▼
                     GIT-027 (NAPI) ◀── WAITS FOR ALL 3
```

## Related Stories

| Story | Relationship | Notes |
|-------|--------------|-------|
| GIT-023 | **Depends On** | List/inspect may be used before discard |
| GIT-015 | Uses (Done) | Provides `abort_session()` primitive |
| GIT-024 | Parallel | Both depend on GIT-023, can work concurrently |
| GIT-026 | Parallel | Both depend on GIT-023, can work concurrently |
| GIT-027 | **Required By** | NAPI bindings expose discard operation |
