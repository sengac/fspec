# AST Research: Session Completion and Status Derivation

## Summary

Research performed to understand the current session management implementation and identify where to add session status derivation and completion behavior changes.

## Key Files Analyzed

### 1. codelet/napi/src/session_manager.rs (299KB)

**BackgroundSession struct** (lines ~984-1058):
```rust
/// GIT-019: Path to worktree for isolated sessions
/// Only set when session was created with isolated=true
pub worktree_path: Option<PathBuf>,

/// GIT-019: Base commit SHA for isolated sessions
pub base_commit: Option<String>,
```

**effective_cwd() method** (lines ~1058-1070):
- Returns worktree path for isolated sessions
- Returns project root for non-isolated sessions

**SessionStatus enum** (lines ~84-96):
```rust
pub enum SessionStatus {
    Idle = 0,
    Running = 1,
    Interrupted = 2,
    Paused = 3,
    Compacting = 4,
}
```
Note: This is the RUNTIME status, NOT the derived status we need for GIT-022.

### 2. codelet/git/src/session_result.rs

**SessionResult struct** (lines 16-32):
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

**get_session_diff()** (lines 45-115):
- Compares base_commit tree against worktree working directory
- Returns SessionResult with file changes
- Used to determine if worktree has uncommitted changes

### 3. codelet/git/src/worktree.rs

**WorktreeInfo struct** (lines 16-27):
```rust
pub struct WorktreeInfo {
    pub session_id: String,
    pub path: PathBuf,
    pub head_commit: String,
    pub is_detached: bool,
}
```

**list_worktrees()** (lines 148-192):
- Lists all worktrees in `.fspec/worktrees/`
- Returns `Vec<WorktreeInfo>`
- Reads HEAD commit from `.git/worktrees/<session_id>/HEAD`

**remove_worktree()** (lines 115-137):
- Removes worktree directory and git metadata
- Currently called by `apply_session_changes()` and `abort_session()`

### 4. codelet/git/src/isolated_session.rs

**IsolatedSessionInfo struct** (lines 17-25):
```rust
pub struct IsolatedSessionInfo {
    pub project: PathBuf,
    pub worktree_path: Option<PathBuf>,
    pub base_commit: Option<String>,
}
```

**effective_cwd()** (lines 83-87):
- Returns worktree path if isolated, otherwise project root

## Implementation Plan

### New Types Needed

**SessionStatus enum** (NOT the runtime status - a new derived status):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedSessionStatus {
    /// Session is in BackgroundSession's active map
    Active,
    /// Worktree exists, not active, HAS uncommitted changes
    PendingMerge,
    /// Worktree exists, not active, NO uncommitted changes
    Clean,
    /// Worktree exists but no session record (manifest missing or terminated)
    Orphaned,
}
```

**SessionManifest struct**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionManifest {
    pub session_id: String,
    pub project_root: PathBuf,
    pub worktree_path: Option<PathBuf>,
    pub base_commit: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub terminated: bool,
}
```

### Files to Modify

1. **codelet/git/src/session_manager.rs** (NEW FILE)
   - Add `DerivedSessionStatus` enum
   - Add `SessionManifest` struct
   - Add `derive_session_status()` function
   - Add manifest read/write functions

2. **codelet/git/src/lib.rs**
   - Export new session_manager module

3. **codelet/napi/src/session_manager.rs**
   - Modify session completion to NOT cleanup worktree
   - Add manifest creation on session start
   - Add manifest update on session completion (set completed_at)

### Status Derivation Logic

```rust
pub fn derive_session_status(
    repo_path: &Path,
    session_id: &str,
    active_sessions: &HashSet<String>,
) -> Result<DerivedSessionStatus> {
    // 1. Check active map first
    if active_sessions.contains(session_id) {
        return Ok(DerivedSessionStatus::Active);
    }
    
    // 2. Check if worktree exists
    let worktrees = list_worktrees(repo_path)?;
    let worktree = worktrees.iter()
        .find(|w| w.session_id == session_id);
    
    if worktree.is_none() {
        return Err(SessionError::NotFound);
    }
    
    // 3. Check session manifest
    let manifest_path = get_manifest_path(session_id);
    if !manifest_path.exists() || is_terminated(&manifest_path)? {
        return Ok(DerivedSessionStatus::Orphaned);
    }
    
    // 4. Check for changes using get_session_diff()
    let diff = get_session_diff(repo_path, session_id)?;
    if diff.files_changed.is_empty() 
        && diff.files_added.is_empty() 
        && diff.files_deleted.is_empty() 
    {
        Ok(DerivedSessionStatus::Clean)
    } else {
        Ok(DerivedSessionStatus::PendingMerge)
    }
}
```

### Session Completion Change

Current: Session completion may cleanup worktree
Required: Session completion should:
1. Stop the agent loop
2. Update manifest with `completed_at` timestamp
3. **NOT cleanup worktree** (leave for user review)

Worktree cleanup should happen via:
- `merge_session()` (GIT-024) - apply changes and cleanup
- `discard_session()` (GIT-025) - abort and cleanup

## Dependencies

- **GIT-019** (done): Provides worktree tracking in BackgroundSession
- **GIT-015** (done): Provides `get_session_diff()` for change detection

## Test Strategy

Tests will be in `codelet/git/tests/session_status_test.rs`:
1. Test Active status when session is in active map
2. Test PendingMerge status when worktree has changes
3. Test Clean status when worktree has no changes
4. Test Orphaned status when manifest is missing
5. Test Orphaned status when manifest has terminated=true
6. Test that session completion does NOT cleanup worktree
7. Test that session completion updates manifest with completed_at
