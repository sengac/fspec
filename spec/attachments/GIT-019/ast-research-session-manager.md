# AST Research: Session Manager Analysis

## Overview
Research performed to understand the existing session_manager.rs structure for implementing isolated session support (GIT-019).

## Key Findings

### BackgroundSession struct location
```
/home/rquast/projects/fspec/codelet/napi/src/session_manager.rs:861 - pub struct BackgroundSession
```

The BackgroundSession struct (line 861) contains:
- `id: Uuid` - Session ID
- `name: RwLock<String>` - Session name
- `project: String` - Project path (this is our project_root)
- Various other fields for status, channels, etc.

### Session Creation Methods
```
Line 3929: pub async fn create_session()
Line 3940: pub async fn create_session_with_id() - PRIMARY TARGET
Line 4045: pub async fn create_watcher_session_with_id()
```

The `create_session_with_id()` method at line 3940 is the main entry point for session creation that we need to modify.

### BackgroundSession impl block
```
/home/rquast/projects/fspec/codelet/napi/src/session_manager.rs:952 - impl BackgroundSession
```

The `new()` constructor is defined here (line 954). We need to:
1. Add `worktree_path: Option<PathBuf>` field to struct
2. Add `base_commit: Option<String>` field to struct
3. Modify constructor to accept these new fields
4. Add `effective_cwd()` method to return worktree_path or project root

### Worktree Module (GIT-014)
```
/home/rquast/projects/fspec/codelet/git/src/worktree.rs
```

Key functions available:
- `create_worktree(repo_path, session_id)` - Creates worktree at HEAD
- `create_worktree_at_ref(repo_path, session_id, commit_ref)` - Creates at specific ref
- `FSPEC_WORKTREES_DIR` constant = ".fspec/worktrees"

The `WorktreeCreateResult` struct provides:
- `info.path` - Worktree path (PathBuf)
- `base_commit` - The commit SHA

## Implementation Plan

1. Add fields to `BackgroundSession`:
   ```rust
   /// Path to worktree for isolated sessions (None if non-isolated)
   pub worktree_path: Option<PathBuf>,
   /// Base commit for isolated sessions
   pub base_commit: Option<String>,
   ```

2. Modify `BackgroundSession::new()` to accept and store these fields

3. Add `effective_cwd()` method:
   ```rust
   pub fn effective_cwd(&self) -> PathBuf {
       self.worktree_path.clone().unwrap_or_else(|| PathBuf::from(&self.project))
   }
   ```

4. Modify `create_session_with_id()` to:
   - Accept `isolated: bool` parameter (default false)
   - If isolated, call `create_worktree()` and store result
   - Pass worktree info to `BackgroundSession::new()`
