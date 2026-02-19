# GIT-019: Isolated Session Creation and effective_cwd

## Overview

This story adds the foundation for isolated session support in BackgroundSession. It introduces the `isolated` parameter for session creation and implements the `effective_cwd()` method that returns the appropriate working directory based on isolation mode.

## Problem Statement

BackgroundSession currently has no concept of isolation. All sessions operate in the main project directory, which means multiple AI agents cannot work in parallel without file conflicts.

## Solution

1. Add `isolated: bool` parameter to session creation
2. When `isolated=true`, create a worktree via GIT-014 primitives
3. Track `worktree_path` and `base_commit` in session state
4. Implement `effective_cwd()` method that returns:
   - Worktree path if session is isolated
   - Project root if session is non-isolated (default)

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| Create isolated session with worktree | When isolated=true, worktree created at `.fspec/worktrees/<session-id>/` |
| Create non-isolated session without worktree | When isolated=false, no worktree created |
| Default session creation is non-isolated | Omitting isolation param defaults to non-isolated |
| effective_cwd returns worktree path for isolated session | Isolated sessions use worktree as cwd |
| effective_cwd returns project root for non-isolated session | Non-isolated sessions use project root |
| Create isolated session fails if worktree already exists | Error if worktree already exists for session ID |

## Implementation Location

### Primary Files to Modify

```
codelet/napi/src/session_manager.rs
├── Add IsolatedSessionConfig struct
├── Add worktree_path: Option<PathBuf> to BackgroundSession
├── Add base_commit: Option<String> to BackgroundSession  
├── Add effective_cwd() method to BackgroundSession
└── Modify create_session() to accept isolated parameter
```

### Dependencies Used

```
codelet/git/src/worktree.rs (GIT-014)
├── create_worktree(repo_path, session_id) -> WorktreeCreateResult
├── FSPEC_WORKTREES_DIR constant
└── WorktreeExists error
```

## API Design

### Rust API

```rust
/// Configuration for isolated session creation
pub struct IsolatedSessionConfig {
    /// Whether to create an isolated worktree
    pub isolated: bool,
    /// Optional commit ref to base worktree on (defaults to HEAD)
    pub base_ref: Option<String>,
}

impl BackgroundSession {
    /// Returns the effective working directory for this session
    /// - For isolated sessions: worktree path
    /// - For non-isolated sessions: project root
    pub fn effective_cwd(&self) -> PathBuf {
        self.worktree_path.clone().unwrap_or_else(|| self.project_root.clone())
    }
}
```

### NAPI Binding (exposed in GIT-027)

```typescript
interface CreateSessionOptions {
  isolated?: boolean;  // defaults to false
  baseRef?: string;    // defaults to HEAD
}

interface SessionInfo {
  sessionId: string;
  worktreePath?: string;  // only set if isolated
  baseCommit?: string;    // only set if isolated
  isIsolated: boolean;
}
```

## Test Strategy

Tests should be written in `codelet/napi/tests/isolated_session_creation_test.rs`:

1. **Unit tests** for effective_cwd() logic
2. **Integration tests** that create actual worktrees via GIT-014 primitives
3. **Error case tests** for worktree-already-exists scenario

## Dependencies

- **GIT-014** (done): Provides `create_worktree()` primitive
- **GIT-015** (done): Provides session result collection
- **GIT-017** (done): Provides ghost commit primitives

## Downstream Dependencies

The following stories depend on GIT-019:

- **GIT-020**: File operations use effective_cwd()
- **GIT-021**: Checkpoint integration needs worktree path
- **GIT-022**: Status derivation needs worktree tracking
- **GIT-027**: NAPI bindings expose this functionality

## Acceptance Criteria Checklist

- [ ] `isolated` parameter accepted in session creation
- [ ] Worktree created at `.fspec/worktrees/<session-id>/` when isolated=true
- [ ] `worktree_path` tracked in session state
- [ ] `base_commit` tracked in session state
- [ ] `effective_cwd()` returns worktree path for isolated sessions
- [ ] `effective_cwd()` returns project root for non-isolated sessions
- [ ] Default behavior is non-isolated (backward compatible)
- [ ] WorktreeExists error returned if worktree already exists
- [ ] All tests pass

---

## Next Steps

After completing GIT-019, the following stories can be worked on in parallel:

| Story | Title | Why It's Next |
|-------|-------|---------------|
| **GIT-020** | Isolated Session File Operations | Uses `effective_cwd()` to route file operations to worktree |
| **GIT-021** | Session Checkpoint Integration | Connects ghost commits to session checkpoints using worktree_path |
| **GIT-022** | Session Completion and Status Derivation | Needs worktree tracking to derive session status |

All three depend on GIT-019 and can be worked on concurrently.

## Story Dependency Graph

```
GIT-018 (Parent)
    │
    └── GIT-019 (This Story) ◀── ENTRY POINT
            │
            ├── GIT-020 (File Operations)
            │
            ├── GIT-021 (Checkpoints)
            │
            └── GIT-022 (Status Derivation)
                    │
                    └── GIT-023 (List/Inspect)
                            │
                            ├── GIT-024 (Merge)
                            │
                            ├── GIT-025 (Discard)
                            │
                            └── GIT-026 (Orphan Pruning)
                                    │
                                    └── GIT-027 (NAPI Bindings) ◀── FINAL STORY
```
