# AST Research: Session Worktree Manager

## Research Date
2026-02-18

## Objective
Understand existing codebase patterns for implementing the Session Worktree Manager (GIT-018).

## Dependencies Analyzed

### GIT-014: Worktree Primitives (codelet/git/src/worktree.rs)

**Public API:**
- `create_worktree(repo_path, session_id)` → `WorktreeCreateResult`
- `create_worktree_at_ref(repo_path, session_id, commit_ref)` → `WorktreeCreateResult`
- `remove_worktree(repo_path, session_id)` → `Result<()>`
- `list_worktrees(repo_path)` → `Vec<WorktreeInfo>`

**Structs:**
- `WorktreeInfo { session_id, path, head_commit, is_detached }`
- `WorktreeCreateResult { info: WorktreeInfo, base_commit, created_at }`

**Key Constants:**
- `FSPEC_WORKTREES_DIR = ".fspec/worktrees"` - where worktrees are stored

### GIT-015: Session Result Operations (codelet/git/src/session_result.rs)

**Public API:**
- `get_session_diff(repo_path, session_id)` → `SessionResult`
- `apply_session_changes(repo_path, session_id)` → `Result<()>`
- `abort_session(repo_path, session_id)` → `Result<()>`

**Structs:**
- `SessionResult { session_id, diff, files_changed, files_added, files_deleted, base_commit }`

**Key Behaviors:**
- `get_session_diff` - non-destructive, returns diff for review
- `apply_session_changes` - copies files, removes worktree, detects conflicts
- `abort_session` - alias for `remove_worktree`

### NAPI Bindings Pattern (codelet/napi/src/git.rs)

**Pattern:**
1. Define `*Js` struct with `#[napi(object)]` for return types
2. Use `#[napi]` on functions
3. Convert Rust types to JS-friendly types (PathBuf → String, chrono → RFC3339)
4. Map errors with `.map_err(|e| napi::Error::from_reason(e.to_string()))`

**Existing Bindings:**
- Worktree: `create_worktree`, `create_worktree_at_ref`, `remove_worktree`, `list_worktrees`
- Session: `get_session_diff`, `apply_session_changes`, `abort_session`

## Implementation Plan for GIT-018

### New Module: codelet/git/src/session_manager.rs

**Session Status Enum:**
```rust
pub enum SessionStatus {
    Active,       // Session process is running
    PendingMerge, // Completed with changes
    Clean,        // Completed with no changes
    Orphaned,     // Worktree exists but no session manifest
}
```

**Session Info Struct:**
```rust
pub struct SessionInfo {
    pub session_id: String,
    pub status: SessionStatus,
    pub worktree_path: PathBuf,
    pub base_commit: String,
    pub files_changed: usize,
    pub created_at: DateTime<Utc>,
}
```

**Functions to Implement:**

1. `list_sessions(repo_path, filter: Option<SessionFilter>)` → `Vec<SessionInfo>`
   - List worktrees from `.fspec/worktrees/`
   - Check session manifests at `~/.fspec/sessions/<session_id>.json`
   - Derive status from manifest state + diff presence
   - Optional filter: all, active, pending_merge, clean, orphaned

2. `inspect_session(repo_path, session_id)` → `SessionResult`
   - Alias for `get_session_diff` with additional validation

3. `merge_session(repo_path, session_id)` → `Result<()>`
   - Alias for `apply_session_changes` with pre-validation

4. `discard_session(repo_path, session_id)` → `Result<()>`
   - Alias for `abort_session`

5. `prune_orphaned(repo_path)` → `PruneResult { count: usize, pruned: Vec<String> }`
   - Find all orphaned sessions
   - Remove each worktree
   - Return count and list of pruned session IDs

### Session Manifest Integration

Session manifests are stored at `~/.fspec/sessions/<session_id>.json` (per GIT-014 rule [7]).

Need to define or import `SessionManifest` struct:
```rust
pub struct SessionManifest {
    pub session_id: String,
    pub worktree_path: Option<PathBuf>,
    pub status: String, // "active", "completed", "terminated"
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

**Orphan Detection:**
- Worktree exists in `.fspec/worktrees/<session_id>/`
- AND either:
  - No manifest at `~/.fspec/sessions/<session_id>.json`
  - OR manifest has `status: "terminated"`

### NAPI Bindings to Add

```rust
#[napi(object)]
pub struct SessionInfoJs {
    pub session_id: String,
    pub status: String, // "active" | "pending_merge" | "clean" | "orphaned"
    pub worktree_path: String,
    pub base_commit: String,
    pub files_changed: u32,
    pub created_at: String, // ISO 8601
}

#[napi(object)]
pub struct PruneResultJs {
    pub count: u32,
    pub pruned: Vec<String>,
}

#[napi]
pub fn list_sessions(repo_path: String, filter: Option<String>) -> napi::Result<Vec<SessionInfoJs>>;

#[napi]
pub fn inspect_session(repo_path: String, session_id: String) -> napi::Result<SessionResultJs>;

#[napi]
pub fn merge_session(repo_path: String, session_id: String) -> napi::Result<()>;

#[napi]
pub fn discard_session(repo_path: String, session_id: String) -> napi::Result<()>;

#[napi]
pub fn prune_orphaned(repo_path: String) -> napi::Result<PruneResultJs>;
```

## Files to Modify/Create

1. **Create:** `codelet/git/src/session_manager.rs`
2. **Modify:** `codelet/git/src/lib.rs` - add `pub mod session_manager;` and re-export
3. **Modify:** `codelet/napi/src/git.rs` - add NAPI bindings for session manager

## Key Insights

1. Most heavy lifting already done in GIT-014 and GIT-015
2. Session manager is primarily orchestration + session manifest integration
3. Need to define session manifest structure (or import from existing code)
4. Status derivation is computed at list time, not stored separately
5. NAPI pattern is well-established - follow existing conventions
