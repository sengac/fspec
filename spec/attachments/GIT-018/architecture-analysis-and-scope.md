# GIT-018 Architecture Analysis and Scope Definition

## Date: 2026-02-18

## Problem Discovery

When analyzing the gitoxide integration epic, we discovered a critical gap:

### What Exists (Layer 1 - Primitives)

**GIT-014 (DONE):** `codelet/git/src/worktree.rs`
- `create_worktree(repo_path, session_id)` → `WorktreeCreateResult`
- `create_worktree_at_ref(repo_path, session_id, commit_ref)` → `WorktreeCreateResult`
- `remove_worktree(repo_path, session_id)` → `Result<()>`
- `list_worktrees(repo_path)` → `Vec<WorktreeInfo>`

**GIT-015 (DONE):** `codelet/git/src/session_result.rs`
- `get_session_diff(repo_path, session_id)` → `SessionResult`
- `apply_session_changes(repo_path, session_id)` → `Result<()>` (copies files + removes worktree)
- `abort_session(repo_path, session_id)` → `Result<()>` (alias for remove_worktree)

**GIT-017 (VALIDATING):** `codelet/git/src/ghost_commit.rs`
- `create_ghost_commit(repo_path, work_unit_id, checkpoint_name)` → `GhostCheckpoint`
- `restore_ghost_commit(repo_path, work_unit_id, checkpoint_name, force)` → `RestoreResult`
- `list_ghost_checkpoints(repo_path, work_unit_id)` → `Vec<String>`
- `delete_ghost_checkpoint(repo_path, work_unit_id, checkpoint_name)` → `Result<()>`

**NAPI Bindings:** `codelet/napi/src/git.rs`
- All the above functions are exposed to TypeScript

### What's Missing (Layer 2 - Integration)

The `BackgroundSession` in `codelet/napi/src/session_manager.rs` has **NO integration** with worktrees:
- No `git_worktree` field
- No `effective_cwd()` method
- No `isolated` parameter on session creation
- Tools don't know about worktrees

The architecture doc (`docs/architecture/multi-session-git.md`) describes this integration but it was never implemented.

### The Gap

```
Primitives (GIT-014/015/017)  →  ???  →  BackgroundSession  →  User
         ↑                        ↑
      EXISTS               MISSING (was GIT-018's assumed prerequisite)
```

## Solution: Expand GIT-018 Scope

GIT-018 should include BOTH:
1. **Integration** - Connect primitives to BackgroundSession
2. **Management** - UI for listing, merging, discarding sessions

### New Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│  GIT-018: BackgroundSession Worktree Integration + Management   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  PART A: Integration Layer                                       │
│  ─────────────────────────────────────────────────────────────  │
│  1. Session Creation:                                            │
│     - Add `isolated: bool` parameter to createSession()         │
│     - If isolated=true: call create_worktree()                  │
│     - Track worktree info in session metadata                   │
│                                                                  │
│  2. Session Execution:                                           │
│     - effective_cwd() → worktree path if isolated, else project │
│     - Tools use effective_cwd() for all file operations         │
│     - (Tool integration may be separate card if complex)        │
│                                                                  │
│  3. Session Checkpoints:                                         │
│     - checkpoint() → create_ghost_commit() in worktree          │
│     - restore() → restore_ghost_commit()                        │
│                                                                  │
│  4. Session Completion:                                          │
│     - Session ends → worktree stays for review                  │
│     - Status derived from session state + worktree existence    │
│                                                                  │
│  PART B: Management Layer                                        │
│  ─────────────────────────────────────────────────────────────  │
│  1. list_sessions(filter?) → sessions with derived status       │
│  2. inspect_session(id) → diff without side effects             │
│  3. merge_session(id) → apply changes, remove worktree          │
│  4. discard_session(id) → remove worktree without applying      │
│  5. prune_orphaned() → clean stale worktrees                    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Session Status Derivation

Status is COMPUTED, not stored:

| Status | Condition |
|--------|-----------|
| `active` | Session ID exists in BackgroundSession's active sessions map |
| `pending_merge` | Worktree exists + NOT active + `get_session_diff()` returns changes |
| `clean` | Worktree exists + NOT active + `get_session_diff()` returns no changes |
| `orphaned` | Worktree exists + no record in active sessions + no session manifest |

### Key Design Decisions

1. **No SessionManifest changes** - Status is derived at query time
2. **Worktree path:** `.fspec/worktrees/<session-id>/` (per GIT-014)
3. **Session tracking:** Active sessions tracked in memory by BackgroundSession
4. **Orphan detection:** Worktree exists but session_id not found anywhere
5. **Pure gitoxide:** All operations use Rust gix library, no git CLI

### Integration Points with BackgroundSession

The existing `BackgroundSession` struct needs:

```rust
// In codelet/napi/src/session_manager.rs

pub struct BackgroundSession {
    // ... existing fields ...
    
    // NEW: Track if this session uses an isolated worktree
    pub worktree_path: Option<PathBuf>,
    pub base_commit: Option<String>,
}

impl BackgroundSession {
    // NEW: Get working directory for tools
    pub fn effective_cwd(&self) -> &Path {
        self.worktree_path.as_deref()
            .unwrap_or_else(|| Path::new(&self.project))
    }
}
```

### Files to Modify/Create

1. **Create:** `codelet/git/src/session_manager.rs` - Rust orchestration layer
2. **Modify:** `codelet/napi/src/session_manager.rs` - Add worktree support to BackgroundSession
3. **Modify:** `codelet/napi/src/git.rs` - Add NAPI bindings for session management
4. **Possibly Modify:** Tool execution paths to use effective_cwd (may be separate card)

### NAPI Functions to Expose

```typescript
// Session management
listSessions(repoPath: string, filter?: string): SessionInfo[]
inspectSession(repoPath: string, sessionId: string): SessionResult
mergeSession(repoPath: string, sessionId: string): void
discardSession(repoPath: string, sessionId: string): void
pruneOrphaned(repoPath: string): PruneResult

// Types
interface SessionInfo {
  sessionId: string;
  status: 'active' | 'pending_merge' | 'clean' | 'orphaned';
  worktreePath: string;
  baseCommit: string;
  filesChanged: number;
  createdAt: string;
}

interface PruneResult {
  count: number;
  pruned: string[];
}
```

### Dependencies

- GIT-014 (DONE): Worktree primitives
- GIT-015 (DONE): Session result primitives  
- GIT-017 (VALIDATING): Ghost commit primitives (for checkpoint integration)

### Estimate

This is a **13 point** story - at the upper limit of acceptable size.

Breakdown:
- Part A (Integration): ~8 points
- Part B (Management): ~5 points

Could potentially split, but the two parts are tightly coupled - integration without management leaves no way to merge results, management without integration has nothing to manage.

### Out of Scope (Future Cards)

1. **Tool CWD integration** - Making all tools use effective_cwd() may be complex enough for its own card
2. **Session persistence updates** - If we decide to persist worktree info in SessionManifest
3. **Auto-merge on session end** - Currently manual merge is required
4. **Conflict resolution UI** - Current design returns error with conflict list

### References

- Architecture doc: `docs/architecture/multi-session-git.md`
- Worktree primitives: `codelet/git/src/worktree.rs`
- Session result primitives: `codelet/git/src/session_result.rs`
- BackgroundSession: `codelet/napi/src/session_manager.rs` (6800+ lines)
- Existing NAPI git bindings: `codelet/napi/src/git.rs`
