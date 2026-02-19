# GIT-024: Session Manager Merge Operations

## Overview

This story implements the merge operation that applies session changes to the main worktree. It uses the `apply_session_changes()` primitive from GIT-015 with proper conflict detection.

## Problem Statement

After reviewing a session's changes via inspection, users need to apply those changes to the main project. This must handle conflicts gracefully and clean up the worktree after successful merge.

## Solution

1. Implement `merge_session(id)` that:
   - Detects conflicts with main worktree
   - Applies changes (copy files, delete files)
   - Removes session worktree on success
2. Return detailed conflict information on failure
3. Leave worktree intact on conflict

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| Merge session changes to main worktree | Applies all changes, removes worktree |
| Merge session applies added files | New files copied to main |
| Merge session applies deleted files | Deleted files removed from main |
| Merge session fails when main has conflicting changes | ConflictError with file list |
| Merge session fails when added file conflicts with main | New file exists in main with different content |
| Merge multiple pending sessions in chosen order | User controls merge order |
| Merge clean session removes worktree | Even empty sessions can be merged |

## Implementation Location

### Add to Session Manager Module

```
codelet/git/src/session_manager.rs
├── merge_session(repo_path, session_id) -> Result<MergeResult>
└── MergeResult struct
```

### Uses GIT-015 Primitives

```
codelet/git/src/session_result.rs (GIT-015)
├── apply_session_changes(repo_path, session_id) -> Result<()>
└── ConflictError with files list
```

## API Design

### MergeResult Struct

```rust
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

### Merge Function

```rust
/// Merge session changes to main worktree
/// 
/// # Algorithm
/// 1. Get session diff to know what changed
/// 2. Call apply_session_changes() which:
///    - Detects conflicts
///    - Copies modified/added files
///    - Deletes removed files
///    - Removes worktree on success
/// 3. Return MergeResult or ConflictError
/// 
/// # Errors
/// - WorktreeNotFound if session doesn't exist
/// - ConflictError if main has conflicting changes
pub fn merge_session(
    repo_path: &Path,
    session_id: &str,
) -> Result<MergeResult> {
    // Get diff first (for return value)
    let diff = get_session_diff(repo_path, session_id)?;
    
    // Apply changes (this handles conflicts and cleanup)
    apply_session_changes(repo_path, session_id)?;
    
    // Return what was merged
    Ok(MergeResult {
        session_id: session_id.to_string(),
        files_modified: diff.files_changed,
        files_added: diff.files_added,
        files_deleted: diff.files_deleted,
    })
}
```

### Conflict Error Handling

```rust
match merge_session(repo_path, session_id) {
    Ok(result) => {
        println!("Merged {} files", 
            result.files_modified.len() + 
            result.files_added.len() + 
            result.files_deleted.len()
        );
    }
    Err(GitError::ConflictError { files }) => {
        eprintln!("Merge failed: {} conflicting files", files.len());
        for file in &files {
            eprintln!("  - {}", file);
        }
        // Worktree is still intact - user can resolve and retry
    }
    Err(e) => return Err(e),
}
```

## Conflict Detection

Conflicts occur when:
1. **Modified file conflict**: File modified in both session and main since base_commit
2. **Added file conflict**: File added in session but already exists in main with different content
3. **Delete conflict**: File deleted in session but modified in main since base_commit

```rust
fn detect_conflicts(
    base_tree: &HashMap<String, Vec<u8>>,
    session_files: &HashMap<String, Vec<u8>>,
    main_files: &HashMap<String, Vec<u8>>,
) -> Vec<String> {
    let mut conflicts = Vec::new();
    
    // Check modified files
    for (path, base_content) in base_tree {
        let session_changed = session_files.get(path)
            .map(|c| c != base_content)
            .unwrap_or(true); // deleted = changed
        
        let main_changed = main_files.get(path)
            .map(|c| c != base_content)
            .unwrap_or(false);
        
        if session_changed && main_changed {
            conflicts.push(path.clone());
        }
    }
    
    // Check added files
    for path in session_files.keys() {
        if !base_tree.contains_key(path) && main_files.contains_key(path) {
            if session_files.get(path) != main_files.get(path) {
                conflicts.push(path.clone());
            }
        }
    }
    
    conflicts
}
```

## Test Strategy

Tests in `codelet/git/tests/session_manager_merge_test.rs`:

1. **Simple merge**: Apply changes, verify in main, worktree removed
2. **Added files**: New files appear in main
3. **Deleted files**: Files removed from main
4. **Modified conflict**: Both session and main modify same file
5. **Added conflict**: Session adds file that exists in main
6. **Multiple sessions**: Merge in user-chosen order
7. **Clean session**: Merge empty session just removes worktree

## Dependencies

- **GIT-023** (required): List/inspect before merge
- **GIT-015** (done): Provides `apply_session_changes()` primitive

## Downstream Dependencies

- **GIT-027**: NAPI bindings expose merge operation

## Acceptance Criteria Checklist

- [ ] `merge_session()` applies changes to main worktree
- [ ] Modified files copied correctly
- [ ] Added files created in main
- [ ] Deleted files removed from main
- [ ] Worktree removed after successful merge
- [ ] ConflictError returned with file list on conflict
- [ ] Worktree intact after conflict
- [ ] Clean sessions can be merged (just removes worktree)
- [ ] All tests pass

---

## Next Steps

GIT-024 is a **parallel story** with GIT-025 and GIT-026. Once complete:

| Action | Description |
|--------|-------------|
| **Mark Done** | Move GIT-024 to `done` status |
| **Verify Integration** | Merge operations work correctly |
| **Check GIT-027** | If GIT-025 and GIT-026 are also done, GIT-027 can start |

## Story Dependency Graph

```
GIT-023 (List/Inspect)
    │
    ├── GIT-024 (This Story) ◀── MERGE OPERATIONS
    │           │
    │           └────────────┐
    │                        │
    ├── GIT-025 (Discard)    │
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
| GIT-023 | **Depends On** | List/inspect before merge |
| GIT-015 | Uses (Done) | Provides `apply_session_changes()` primitive |
| GIT-025 | Parallel | Both depend on GIT-023, can work concurrently |
| GIT-026 | Parallel | Both depend on GIT-023, can work concurrently |
| GIT-027 | **Required By** | NAPI bindings expose merge operation |
