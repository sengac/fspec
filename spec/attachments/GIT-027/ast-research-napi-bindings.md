# AST Research: NAPI Bindings for Session Worktree Operations

## Existing NAPI Functions in codelet/napi/src/git.rs

Current bindings (to be extended):

| Function | Line | Signature |
|----------|------|-----------|
| `create_worktree` | 103 | `pub fn create_worktree(repo_path: String, session_id: String) -> napi::Result<WorktreeCreateResultJs>` |
| `create_worktree_at_ref` | 124 | `pub fn create_worktree_at_ref(repo_path: String, session_id: String, commit_ref: Option<String>) -> napi::Result<WorktreeCreateResultJs>` |
| `list_worktrees` | 161 | `pub fn list_worktrees(repo_path: String) -> napi::Result<Vec<WorktreeInfoJs>>` |
| `get_session_diff` | 207 | `pub fn get_session_diff(repo_path: String, session_id: String) -> napi::Result<SessionResultJs>` |
| `apply_session_changes` | 230 | `pub fn apply_session_changes(repo_path: String, session_id: String) -> napi::Result<()>` |
| `abort_session` | 240 | `pub fn abort_session(repo_path: String, session_id: String) -> napi::Result<()>` |

## Rust Functions to Wrap in session_status.rs

Target functions for NAPI exposure:

| Function | Line | Signature |
|----------|------|-----------|
| `list_sessions` | 364 | `pub fn list_sessions(repo_path: impl AsRef<Path>, active_sessions: &HashSet<String>, filter: SessionFilter) -> Result<Vec<SessionInfo>>` |
| `inspect_session` | 436 | `pub fn inspect_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<SessionResult>` |
| `merge_session` | 504 | `pub fn merge_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<MergeResult>` |
| `discard_session` | 577 | `pub fn discard_session(repo_path: impl AsRef<Path>, session_id: &str) -> Result<DiscardResult>` |
| `prune_orphaned` | 694 | `pub fn prune_orphaned(repo_path: impl AsRef<Path>, active_sessions: &HashSet<String>) -> Result<PruneResult>` |

## Existing JS Types (to extend)

```rust
// Line 68-82
pub struct WorktreeCreateResultJs {
    pub session_id: String,
    pub path: String,
    pub head_commit: String,
    pub is_detached: bool,
    pub base_commit: String,
    pub created_at: String,
}

// Line 181-195
pub struct SessionResultJs {
    pub session_id: String,
    pub diff: String,
    pub files_changed: Vec<String>,
    pub files_added: Vec<String>,
    pub files_deleted: Vec<String>,
    pub base_commit: String,
}
```

## New Types Needed

Based on session_status.rs Rust structs:

1. **SessionInfoJs** - wraps `SessionInfo` (line 337-351)
2. **MergeResultJs** - wraps `MergeResult` (line 452-461)
3. **DiscardResultJs** - wraps `DiscardResult` (line 534-542)
4. **PruneResultJs** - wraps `PruneResult` (line 612-618)
5. **CreateIsolatedSessionOptions** - options for createIsolatedSession

## Implementation Pattern

Based on existing NAPI bindings (e.g., `create_worktree`):

```rust
#[napi]
pub fn function_name(
    repo_path: String,
    param: Type,
) -> napi::Result<ReturnTypeJs> {
    codelet_git::function_name(&repo_path, &param)
        .map(Into::into)
        .map_err(|e| napi::Error::from_reason(e.to_string()))
}
```

## Special Consideration: Active Sessions

Functions `list_sessions` and `prune_orphaned` require an `active_sessions: &HashSet<String>` parameter.
This needs to be passed from TypeScript - the NAPI bindings should accept a `Vec<String>` and convert internally.
