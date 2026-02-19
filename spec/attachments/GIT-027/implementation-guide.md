# GIT-027: Session Worktree NAPI Bindings

## Overview

This story exposes all session worktree operations to TypeScript via NAPI-RS bindings. This is the final story that makes all the Rust functionality accessible from the TUI and TypeScript code.

## Problem Statement

All the session worktree functionality has been implemented in Rust (GIT-019 through GIT-026), but TypeScript code (TUI, commands) cannot access it. NAPI bindings are needed to bridge Rust and TypeScript.

## Solution

Add NAPI bindings in `codelet/napi/src/git.rs` for:
1. `createIsolatedSession()` - Create session with worktree
2. `listSessions()` - List sessions with derived status
3. `inspectSession()` - Get session diff
4. `mergeSession()` - Apply changes to main
5. `discardSession()` - Remove without applying
6. `pruneOrphaned()` - Clean up orphaned worktrees

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| NAPI binding exposes createIsolatedSession | Creates isolated session from TypeScript |
| NAPI binding session includes worktree info | worktreePath, baseCommit, isIsolated fields |
| NAPI binding effective_cwd available | effectiveCwd() method accessible |
| NAPI binding exposes listSessions | Returns array of session objects |
| NAPI binding exposes listSessions with filter | Supports status filter |
| NAPI binding exposes inspectSession | Returns diff object |
| NAPI binding exposes mergeSession | Applies changes and returns result |
| NAPI binding mergeSession returns conflict error | Error includes conflicting files |
| NAPI binding exposes discardSession | Removes worktree |
| NAPI binding exposes pruneOrphaned | Returns prune result |

## Implementation Location

### Primary File to Modify

```
codelet/napi/src/git.rs
├── Add createIsolatedSession()
├── Add listSessions() with filter support
├── Add inspectSession()
├── Add mergeSession()
├── Add discardSession()
└── Add pruneOrphaned()
```

### TypeScript Types Generated

```
codelet/napi/index.d.ts (auto-generated)
├── CreateIsolatedSessionOptions
├── SessionInfoJs
├── SessionStatusJs
├── SessionFilterJs
├── MergeResultJs
├── DiscardResultJs
└── PruneResultJs
```

## API Design

### NAPI Object Types

```rust
/// Options for creating an isolated session
#[napi(object)]
pub struct CreateIsolatedSessionOptions {
    /// Session ID to use
    pub session_id: String,
    /// Optional commit ref to base worktree on (defaults to HEAD)
    pub base_ref: Option<String>,
}

/// Session information with derived status
#[napi(object)]
pub struct SessionInfoJs {
    /// Session ID
    pub session_id: String,
    /// Derived status: "active", "pending_merge", "clean", "orphaned"
    pub status: String,
    /// Base commit the worktree was created from
    pub base_commit: String,
    /// Number of files changed
    pub files_changed: u32,
    /// When the session was created (ISO 8601)
    pub created_at: String,
    /// Path to the worktree
    pub worktree_path: String,
}

/// Result of merging a session
#[napi(object)]
pub struct MergeResultJs {
    /// Session ID that was merged
    pub session_id: String,
    /// Files that were modified
    pub files_modified: Vec<String>,
    /// Files that were added
    pub files_added: Vec<String>,
    /// Files that were deleted
    pub files_deleted: Vec<String>,
}

/// Result of discarding a session
#[napi(object)]
pub struct DiscardResultJs {
    /// Session ID that was discarded
    pub session_id: String,
    /// Number of files that were discarded
    pub files_discarded: u32,
}

/// Result of pruning orphaned sessions
#[napi(object)]
pub struct PruneResultJs {
    /// Number of sessions pruned
    pub count: u32,
    /// Session IDs that were pruned
    pub pruned: Vec<String>,
}
```

### NAPI Functions

```rust
/// Create an isolated session with its own worktree
/// 
/// @param repoPath - Path to the git repository
/// @param options - Session creation options
/// @returns WorktreeCreateResult with session info
#[napi]
pub fn create_isolated_session(
    repo_path: String,
    options: CreateIsolatedSessionOptions,
) -> napi::Result<WorktreeCreateResultJs> {
    let result = codelet_git::create_worktree_at_ref(
        &repo_path,
        &options.session_id,
        options.base_ref.as_deref(),
    ).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(WorktreeCreateResultJs { /* ... */ })
}

/// List all session worktrees with status information
/// 
/// @param repoPath - Path to the git repository  
/// @param filter - Optional status filter: "all", "active", "pending_merge", "clean", "orphaned"
/// @returns Array of SessionInfo objects
#[napi]
pub fn list_sessions(
    repo_path: String,
    filter: Option<String>,
) -> napi::Result<Vec<SessionInfoJs>> {
    let filter = parse_filter(filter.as_deref())?;
    let active = get_active_sessions(); // From SessionManager
    
    let sessions = codelet_git::session_manager::list_sessions(
        &repo_path,
        &active,
        filter,
    ).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    
    Ok(sessions.into_iter().map(Into::into).collect())
}

/// Inspect session diff before merging
/// 
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier
/// @returns SessionResult with diff
#[napi]
pub fn inspect_session(
    repo_path: String,
    session_id: String,
) -> napi::Result<SessionResultJs> {
    codelet_git::session_manager::inspect_session(&repo_path, &session_id)
        .map(Into::into)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Merge session changes to main worktree
/// 
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier
/// @returns MergeResult on success
/// @throws Error with conflicting files on conflict
#[napi]
pub fn merge_session(
    repo_path: String,
    session_id: String,
) -> napi::Result<MergeResultJs> {
    codelet_git::session_manager::merge_session(&repo_path, &session_id)
        .map(Into::into)
        .map_err(|e| {
            // Include conflict file list in error message
            napi::Error::from_reason(e.to_string())
        })
}

/// Discard session without applying changes
/// 
/// @param repoPath - Path to the git repository
/// @param sessionId - Session identifier
/// @returns DiscardResult
#[napi]
pub fn discard_session(
    repo_path: String,
    session_id: String,
) -> napi::Result<DiscardResultJs> {
    codelet_git::session_manager::discard_session(&repo_path, &session_id)
        .map(Into::into)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}

/// Prune all orphaned session worktrees
/// 
/// @param repoPath - Path to the git repository
/// @returns PruneResult with count and pruned list
#[napi]
pub fn prune_orphaned(
    repo_path: String,
) -> napi::Result<PruneResultJs> {
    let active = get_active_sessions();
    
    codelet_git::session_manager::prune_orphaned(&repo_path, &active)
        .map(Into::into)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}
```

## TypeScript Usage Examples

```typescript
import {
    createIsolatedSession,
    listSessions,
    inspectSession,
    mergeSession,
    discardSession,
    pruneOrphaned,
} from 'codelet-napi';

// Create an isolated session
const session = createIsolatedSession('/path/to/repo', {
    sessionId: 'feature-auth',
    baseRef: 'main',
});
console.log(`Created worktree at ${session.worktreePath}`);

// List pending sessions
const pending = listSessions('/path/to/repo', 'pending_merge');
for (const s of pending) {
    console.log(`${s.sessionId}: ${s.filesChanged} files changed`);
}

// Inspect before merge
const diff = inspectSession('/path/to/repo', 'feature-auth');
console.log(`Changes:\n${diff.diff}`);

// Merge session
try {
    const result = mergeSession('/path/to/repo', 'feature-auth');
    console.log(`Merged: ${result.filesModified.length} modified`);
} catch (e) {
    console.error(`Conflict: ${e.message}`);
}

// Prune orphaned
const pruned = pruneOrphaned('/path/to/repo');
console.log(`Pruned ${pruned.count} orphaned sessions`);
```

## Test Strategy

Tests in `codelet/napi/tests/session_worktree_napi_test.rs`:

1. **Create isolated session**: NAPI creates worktree correctly
2. **List sessions**: Returns array with status
3. **List with filter**: Filtering works
4. **Inspect session**: Returns diff object
5. **Merge session**: Applies changes
6. **Merge conflict**: Error contains conflict info
7. **Discard session**: Removes worktree
8. **Prune orphaned**: Returns prune result

## Dependencies

- **GIT-019** (required): Isolated session creation
- **GIT-023** (required): List and inspect operations
- **GIT-024** (required): Merge operations
- **GIT-025** (required): Discard operations
- **GIT-026** (required): Prune operations

## Downstream Dependencies

None - this is the final story in the GIT-018 epic.

## Acceptance Criteria Checklist

- [ ] `createIsolatedSession()` NAPI function works
- [ ] `listSessions()` returns array with status
- [ ] `listSessions()` supports filter parameter
- [ ] `inspectSession()` returns diff object
- [ ] `mergeSession()` applies changes
- [ ] `mergeSession()` error includes conflict files
- [ ] `discardSession()` removes worktree
- [ ] `pruneOrphaned()` returns prune result
- [ ] TypeScript types generated correctly
- [ ] All tests pass

---

## Next Steps

GIT-027 is the **FINAL STORY** in the GIT-018 epic. Once complete:

| Action | Description |
|--------|-------------|
| **Mark Done** | Move GIT-027 to `done` status |
| **Mark Parent Done** | Move GIT-018 to `done` status |
| **TUI Integration** | TypeScript TUI can now use session worktree management |
| **End-to-End Testing** | Test full workflow from TypeScript |

## Story Dependency Graph (Complete)

```
GIT-018 (Parent Story)
    │
    └── GIT-019 (Isolated Session Creation) ◀── ENTRY POINT
            │
            ├── GIT-020 (File Operations) - LEAF
            │
            ├── GIT-021 (Checkpoints) - LEAF
            │
            └── GIT-022 (Status Derivation)
                    │
                    └── GIT-023 (List/Inspect) - CENTRAL HUB
                            │
                            ├── GIT-024 (Merge)
                            │
                            ├── GIT-025 (Discard)
                            │
                            └── GIT-026 (Orphan Pruning)
                                    │
                                    └── GIT-027 (This Story) ◀── FINAL (NAPI BINDINGS)
```

## Required Stories (All Must Be Done)

| Story | Title | Status |
|-------|-------|--------|
| GIT-019 | Isolated Session Creation | Required |
| GIT-023 | Session Manager List and Inspect | Required |
| GIT-024 | Session Manager Merge Operations | Required |
| GIT-025 | Session Manager Discard Operations | Required |
| GIT-026 | Orphan Detection and Pruning | Required |

## What This Enables

Once GIT-027 is complete, the following TypeScript features become available:

1. **Background Session Management UI** - TUI can create/list/manage isolated sessions
2. **Session Merge Dialog** - UI for reviewing and merging session changes
3. **Orphan Cleanup Command** - Slash command to prune orphaned worktrees
4. **Parallel AI Agents** - Multiple AI agents can work in isolated worktrees concurrently
