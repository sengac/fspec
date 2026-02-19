# GIT-021: Session Checkpoint Integration

## Overview

This story connects the ghost commit primitives (GIT-017) to session checkpoint operations. When an isolated session creates a checkpoint, it should use ghost commits to capture the worktree state.

## Problem Statement

GIT-017 created ghost commit primitives, but they're not connected to the session lifecycle. Sessions need a `checkpoint()` method that creates ghost commits in the worktree context.

## Solution

1. Add `checkpoint(label)` method to BackgroundSession
2. For isolated sessions, call `create_ghost_commit()` with worktree path
3. For non-isolated sessions, return error (checkpoints require isolation)
4. Add `restore(label)` method to restore from checkpoint
5. Store checkpoints at `refs/fspec-checkpoints/<session-id>/<label>`

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| Isolated session checkpoint creates ghost commit | checkpoint() calls create_ghost_commit() |
| Isolated session checkpoint captures all worktree changes | Staged, unstaged, and untracked files captured |
| Isolated session restore from checkpoint | restore() returns worktree to checkpoint state |
| Parallel sessions have independent checkpoint history | Each session has its own checkpoint namespace |
| Checkpoint fails for non-isolated session | NotIsolated error returned |

## Implementation Location

### Primary Files to Modify

```
codelet/napi/src/session_manager.rs
├── Add checkpoint(&self, label: &str) method
├── Add restore(&self, label: &str) method
├── Add list_checkpoints(&self) method
└── Add NotIsolated error variant
```

### Ghost Commit Integration

```
codelet/git/src/ghost_commit.rs (GIT-017 - already done)
├── create_ghost_commit(dir, work_unit_id, checkpoint_name)
├── restore_ghost_commit(dir, work_unit_id, checkpoint_name, force)
└── list_ghost_checkpoints(dir, work_unit_id)
```

## API Design

### Rust API

```rust
impl BackgroundSession {
    /// Create a checkpoint capturing current worktree state
    /// 
    /// # Errors
    /// - NotIsolated if session is not isolated
    /// - GitError if ghost commit creation fails
    pub fn checkpoint(&self, label: &str) -> Result<GhostCheckpoint> {
        let worktree_path = self.worktree_path.as_ref()
            .ok_or(SessionError::NotIsolated)?;
        
        // Use session_id as the work_unit_id for checkpoint namespace
        create_ghost_commit(worktree_path, &self.session_id, label)
    }
    
    /// Restore worktree to checkpoint state
    pub fn restore(&self, label: &str) -> Result<RestoreResult> {
        let worktree_path = self.worktree_path.as_ref()
            .ok_or(SessionError::NotIsolated)?;
        
        restore_ghost_commit(worktree_path, &self.session_id, label, true)
    }
    
    /// List all checkpoints for this session
    pub fn list_checkpoints(&self) -> Result<Vec<String>> {
        let worktree_path = self.worktree_path.as_ref()
            .ok_or(SessionError::NotIsolated)?;
        
        list_ghost_checkpoints(worktree_path, &self.session_id)
    }
}
```

### Checkpoint Namespace

Checkpoints are stored at:
```
refs/fspec-checkpoints/<session-id>/<label>
```

Example:
```
refs/fspec-checkpoints/abc-123/before-refactor
refs/fspec-checkpoints/abc-123/working-state
refs/fspec-checkpoints/def-456/initial
```

## Test Strategy

Tests in `codelet/napi/tests/session_checkpoint_integration_test.rs`:

1. **Checkpoint creation**: Verify ghost commit created with correct ref
2. **File capture**: Verify staged, unstaged, untracked files all captured
3. **Restore**: Verify worktree returns to exact checkpoint state
4. **Independent history**: Parallel sessions don't share checkpoints
5. **Non-isolated error**: Verify NotIsolated error for non-isolated sessions

## Dependencies

- **GIT-019** (required): Provides worktree_path and session isolation
- **GIT-017** (done): Provides ghost commit primitives

## Downstream Dependencies

- **GIT-027**: NAPI bindings will expose checkpoint operations

## Acceptance Criteria Checklist

- [ ] `checkpoint(label)` method added to BackgroundSession
- [ ] Ghost commit created at `refs/fspec-checkpoints/<session-id>/<label>`
- [ ] All worktree changes captured (staged, unstaged, untracked)
- [ ] `restore(label)` method restores worktree to checkpoint state
- [ ] `list_checkpoints()` returns all checkpoint labels
- [ ] NotIsolated error for non-isolated sessions
- [ ] Parallel sessions have independent checkpoint namespaces
- [ ] All tests pass

---

## Next Steps

GIT-021 is a **parallel story** that provides checkpoint functionality. Once complete:

| Action | Description |
|--------|-------------|
| **Mark Done** | Move GIT-021 to `done` status |
| **Verify Integration** | Session checkpoints create ghost commits correctly |
| **Wait for GIT-027** | NAPI bindings in GIT-027 will expose checkpoint operations to TypeScript |

## Story Dependency Graph

```
GIT-019 (Isolated Session Creation)
    │
    ├── GIT-020 (File Operations)
    │
    ├── GIT-021 (This Story) ◀── CHECKPOINT FUNCTIONALITY
    │
    └── GIT-022 (Status Derivation)
            │
            └── GIT-023 → ... → GIT-027 (NAPI will expose checkpoint ops)
```

## Related Stories

| Story | Relationship | Notes |
|-------|--------------|-------|
| GIT-019 | **Depends On** | Provides worktree_path for checkpoint operations |
| GIT-017 | Uses (Done) | Ghost commit primitives (`create_ghost_commit()`) |
| GIT-020 | Parallel | Works on file operations independently |
| GIT-022 | Parallel | Works on status derivation independently |
| GIT-027 | **Required By** | Will expose checkpoint operations via NAPI |
