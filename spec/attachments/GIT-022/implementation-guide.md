# GIT-022: Session Completion and Status Derivation

## Overview

This story implements the session completion behavior (worktree stays for review) and the status derivation logic that computes session status at query time rather than storing it.

## Problem Statement

When an isolated session completes, the worktree should remain for user review (not auto-cleanup). Additionally, session status needs to be derived dynamically:
- `active`: Session in BackgroundSession's active map
- `pending_merge`: Worktree exists + not active + has changes
- `clean`: Worktree exists + not active + no changes
- `orphaned`: Worktree exists + no session record

## Solution

1. On session completion, leave worktree intact (don't auto-cleanup)
2. Implement `derive_status()` that checks:
   - BackgroundSession active map first
   - Then worktree state via `get_session_diff()`
3. Store session manifest for completed sessions (for orphan detection)

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| Isolated session completion leaves worktree for review | Worktree not cleaned up on completion |
| Isolated session without changes leaves clean worktree | Status becomes `clean` |
| Session with changes transitions to pending_merge | Status becomes `pending_merge` |
| Session without changes transitions to clean | Status becomes `clean` |
| Status derivation checks BackgroundSession map first | Active sessions always show as `active` |
| Status derivation falls back to worktree state | Non-active sessions check worktree |

## Implementation Location

### Primary Files to Modify

```
codelet/napi/src/session_manager.rs
├── Modify session completion to NOT cleanup worktree
├── Add derive_session_status(session_id) function
└── Add SessionStatus enum

codelet/git/src/session_manager.rs (NEW)
├── SessionStatus enum
├── derive_status(repo_path, session_id, active_sessions) function
└── Uses list_worktrees() and get_session_diff()
```

### Session Manifest Storage

```
~/.fspec/sessions/<session-id>.json
├── session_id: string
├── project_root: string
├── worktree_path: Option<string>
├── base_commit: Option<string>
├── created_at: timestamp
├── completed_at: Option<timestamp>
└── terminated: bool
```

## API Design

### Session Status Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    /// Session is in BackgroundSession's active map
    Active,
    /// Worktree exists, not active, has uncommitted changes
    PendingMerge,
    /// Worktree exists, not active, no uncommitted changes
    Clean,
    /// Worktree exists but no session record (manifest missing/terminated)
    Orphaned,
}
```

### Status Derivation Logic

```rust
pub fn derive_session_status(
    repo_path: &Path,
    session_id: &str,
    active_sessions: &HashSet<String>,
) -> Result<SessionStatus> {
    // 1. Check active map first
    if active_sessions.contains(session_id) {
        return Ok(SessionStatus::Active);
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
        return Ok(SessionStatus::Orphaned);
    }
    
    // 4. Check for changes
    let diff = get_session_diff(repo_path, session_id)?;
    if diff.files_changed.is_empty() 
        && diff.files_added.is_empty() 
        && diff.files_deleted.is_empty() 
    {
        Ok(SessionStatus::Clean)
    } else {
        Ok(SessionStatus::PendingMerge)
    }
}
```

### Session Completion Behavior

```rust
impl BackgroundSession {
    pub async fn complete(&mut self) -> Result<()> {
        // Stop the agent loop
        self.stop().await?;
        
        // Update manifest with completed_at timestamp
        if let Some(worktree_path) = &self.worktree_path {
            self.update_manifest_completed()?;
        }
        
        // DO NOT cleanup worktree - leave for user review
        // Worktree cleanup happens via merge_session() or discard_session()
        
        Ok(())
    }
}
```

## Test Strategy

Tests in `codelet/napi/tests/session_completion_status_test.rs`:

1. **Completion leaves worktree**: Verify worktree exists after session completes
2. **Status derivation - active**: Active session returns `Active`
3. **Status derivation - pending_merge**: Completed session with changes returns `PendingMerge`
4. **Status derivation - clean**: Completed session without changes returns `Clean`
5. **Active takes priority**: Active session with changes still shows as `Active`

## Dependencies

- **GIT-019** (required): Provides worktree tracking in session
- **GIT-015** (done): Provides `get_session_diff()` for change detection

## Downstream Dependencies

- **GIT-023**: Uses status derivation for list/inspect operations
- **GIT-026**: Uses status derivation for orphan detection

## Acceptance Criteria Checklist

- [ ] Session completion leaves worktree intact
- [ ] Session manifest created/updated on completion
- [ ] `SessionStatus` enum with Active, PendingMerge, Clean, Orphaned
- [ ] `derive_session_status()` checks active map first
- [ ] Status correctly derived based on worktree changes
- [ ] Active sessions always show as Active regardless of changes
- [ ] All tests pass

---

## Next Steps

GIT-022 **unlocks GIT-023** (List and Inspect). Once complete:

| Story | Title | Why It's Next |
|-------|-------|---------------|
| **GIT-023** | Session Manager List and Inspect | Uses status derivation to show session info |

## Story Dependency Graph

```
GIT-019 (Isolated Session Creation)
    │
    ├── GIT-020 (File Operations) - Parallel
    │
    ├── GIT-021 (Checkpoints) - Parallel
    │
    └── GIT-022 (This Story) ◀── STATUS DERIVATION
            │
            └── GIT-023 (List/Inspect) ◀── NEXT
                    │
                    ├── GIT-024 (Merge)
                    │
                    ├── GIT-025 (Discard)
                    │
                    └── GIT-026 (Orphan Pruning)
                            │
                            └── GIT-027 (NAPI Bindings)
```

## Related Stories

| Story | Relationship | Notes |
|-------|--------------|-------|
| GIT-019 | **Depends On** | Provides worktree tracking in session |
| GIT-015 | Uses (Done) | Provides `get_session_diff()` for change detection |
| GIT-023 | **Unlocks** | Uses status derivation for listing sessions |
| GIT-026 | **Required By** | Uses orphan detection from status derivation |
