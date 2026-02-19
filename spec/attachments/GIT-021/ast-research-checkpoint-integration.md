# AST Research: Session Checkpoint Integration

## Research Summary

Analyzed the codebase to understand how to integrate ghost commits with BackgroundSession for checkpoint operations.

## Files Analyzed

### 1. codelet/git/src/ghost_commit.rs

**Purpose:** Ghost commit primitives for checkpoint creation and restoration.

**Key Functions:**
- `create_ghost_commit(dir: &Path, work_unit_id: &str, checkpoint_name: &str) -> Result<GhostCheckpoint>`
- `restore_ghost_commit(dir: &Path, work_unit_id: &str, checkpoint_name: &str, force: bool) -> Result<RestoreResult>`
- `list_ghost_checkpoints(dir: &Path, work_unit_id: &str) -> Result<Vec<String>>`
- `delete_ghost_checkpoint(dir: &Path, work_unit_id: &str, checkpoint_name: &str) -> Result<()>`

**Key Types:**
```rust
pub struct GhostCheckpoint {
    pub sha: String,
    pub parent_sha: String,
    pub files: Vec<String>,
}

pub struct RestoreResult {
    pub success: bool,
    pub restored_files: Vec<String>,
    pub deleted_files: Vec<String>,
}
```

**Ref Storage Pattern:** `refs/fspec-checkpoints/{work_unit_id}/{checkpoint_name}`

### 2. codelet/napi/src/session_manager.rs

**Purpose:** Session management for AI agent sessions.

**Key Struct:**
```rust
pub struct BackgroundSession {
    pub id: Uuid,
    // ... other fields ...
    
    /// GIT-019: Path to worktree for isolated sessions
    pub worktree_path: Option<PathBuf>,
    
    /// GIT-019: Base commit SHA for isolated sessions
    pub base_commit: Option<String>,
}
```

**Key Methods:**
- `effective_cwd(&self) -> PathBuf` - Returns worktree path if isolated, else project root
- `new(...)` - Constructor, accepts `worktree_path: Option<PathBuf>` and `base_commit: Option<String>`

**Session Manager Singleton:**
```rust
static SESSION_MANAGER: OnceCell<SessionManager> = OnceCell::new();
```

## Integration Plan

### New Methods to Add to BackgroundSession

1. `checkpoint(&self, label: &str) -> Result<GhostCheckpoint, SessionError>`
   - Check `self.worktree_path.is_some()`, return `SessionError::NotIsolated` if None
   - Call `create_ghost_commit(worktree_path, &self.id.to_string(), label)`

2. `restore(&self, label: &str) -> Result<RestoreResult, SessionError>`
   - Check `self.worktree_path.is_some()`, return `SessionError::NotIsolated` if None
   - Call `restore_ghost_commit(worktree_path, &self.id.to_string(), label, true)`

3. `list_checkpoints(&self) -> Result<Vec<String>, SessionError>`
   - Check `self.worktree_path.is_some()`, return `SessionError::NotIsolated` if None
   - Call `list_ghost_checkpoints(worktree_path, &self.id.to_string())`

### New Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session is not isolated - checkpoint operations require an isolated session with worktree")]
    NotIsolated,
    
    #[error("Git error: {0}")]
    GitError(#[from] codelet_git::error::GitError),
}
```

### Import Required

```rust
use codelet_git::ghost_commit::{
    create_ghost_commit, restore_ghost_commit, list_ghost_checkpoints, 
    GhostCheckpoint, RestoreResult
};
```

## Dependencies Verified

- GIT-017 (Ghost commits): ✅ Implemented in codelet/git/src/ghost_commit.rs
- GIT-019 (Isolated sessions): ✅ worktree_path field exists in BackgroundSession

## Test Location

Tests should be in: `codelet/napi/tests/session_checkpoint_integration_test.rs`
