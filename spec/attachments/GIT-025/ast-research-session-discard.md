# AST Research: Session Discard Operations

## Purpose
Research for GIT-025 to understand existing session management code structure.

## Key Functions in session_status.rs

### Public Functions Identified

| Line | Function | Purpose |
|------|----------|---------|
| 108 | `get_sessions_dir()` | Returns path to git-sessions directory |
| 113 | `get_manifest_path(session_id)` | Returns path to manifest JSON |
| 118 | `read_manifest(session_id)` | Read session manifest from disk |
| 135 | `write_manifest(manifest)` | Write session manifest to disk |
| 155 | `delete_manifest(session_id)` | **DELETE** manifest - REUSE FOR DISCARD |
| 183 | `derive_session_status(...)` | Derive Active/PendingMerge/Clean/Orphaned status |
| 250 | `complete_session(session_id)` | Mark session as completed |
| 274 | `create_session_manifest(...)` | Create new session manifest |
| 297 | `terminate_session(session_id)` | Mark session as terminated (orphaned) |
| 364 | `list_sessions(...)` | List all sessions with filters (GIT-023) |
| 434 | `inspect_session(...)` | Get session diff read-only (GIT-023) |
| 502 | `merge_session(...)` | Merge session to main (GIT-024) |

### Key Dependencies from session_result.rs

From session_result.rs (GIT-015):
- `abort_session(repo_path, session_id)` - Line 204 - Removes worktree without applying changes
- `get_session_diff(repo_path, session_id)` - Line 45 - Get diff info

## Implementation Plan for discard_session()

1. **Use existing primitives:**
   - `get_session_diff()` - To count files before discard
   - `derive_session_status()` - To capture previous_status
   - `abort_session()` - To remove the worktree
   - `delete_manifest()` - To clean up the manifest

2. **Add alongside merge_session():**
   - Location: After line 526 (end of merge_session)
   - Follow same pattern as merge_session

3. **DiscardResult struct:**
   - session_id: String
   - files_discarded: usize
   - previous_status: DerivedSessionStatus

## Existing Pattern from merge_session (line 502-526)

```rust
pub fn merge_session(
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> Result<MergeResult> {
    // 1. Get diff first
    let diff = get_session_diff(repo_path, session_id)?;
    
    // 2. Apply changes
    apply_session_changes(repo_path, session_id)?;
    
    // 3. Delete manifest
    delete_manifest(session_id)?;
    
    // 4. Return result
    Ok(MergeResult { ... })
}
```

## discard_session Pattern

```rust
pub fn discard_session(
    repo_path: impl AsRef<Path>,
    session_id: &str,
) -> Result<DiscardResult> {
    // 1. Get diff first (to count files)
    let diff = get_session_diff(repo_path, session_id)?;
    let files_discarded = diff.files_changed.len() 
        + diff.files_added.len() 
        + diff.files_deleted.len();
    
    // 2. Get status before discard
    let previous_status = derive_session_status(
        repo_path, session_id, &HashSet::new())?;
    
    // 3. Remove worktree (abort_session)
    abort_session(repo_path, session_id)?;
    
    // 4. Delete manifest
    delete_manifest(session_id)?;
    
    // 5. Return result
    Ok(DiscardResult { 
        session_id: session_id.to_string(),
        files_discarded,
        previous_status,
    })
}
```
