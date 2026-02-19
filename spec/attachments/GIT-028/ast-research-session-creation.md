# AST Research: Session Creation Functions

## Research Query
Finding existing session creation functions in `codelet/napi/src/session_manager.rs` to understand the pattern for `session_manager_create_isolated()`.

## Findings

### Existing NAPI Bindings (Lines 5609-5628)
```rust
#[napi]
pub async fn session_manager_create(model: String, project: String) -> Result<String>

#[napi]  
pub async fn session_manager_create_with_id(
    session_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<()>
```

### SessionManager Methods (Lines 4041-4151)
```rust
pub async fn create_session(&self, _model: &str, project: &str) -> Result<String>
pub async fn create_session_with_id(&self, id: &str, model: &str, project: &str, name: &str) -> Result<()>
```

### BackgroundSession::new Already Supports Isolation (Lines 4125-4135)
```rust
let session = Arc::new(BackgroundSession::new(
    uuid,
    name.to_string(),
    project.to_string(),
    provider_id,
    model_id,
    inner,
    input_tx,
    None, // GIT-019: worktree_path (non-isolated by default)
    None, // GIT-019: base_commit (non-isolated by default)
));
```

### What's Missing
The NAPI binding that:
1. Calls `codelet_git::create_worktree()` to create worktree at `.fspec/worktrees/<session-id>/`
2. Calls `codelet_git::create_session_manifest()` for orphan detection
3. Passes `Some(worktree_path)` and `Some(base_commit)` to `BackgroundSession::new()`

### Implementation Plan
Add new function:
```rust
#[napi]
pub async fn session_manager_create_isolated(
    session_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<IsolatedSessionResultJs>
```

Which internally:
1. Creates worktree: `let result = codelet_git::create_worktree(&project, &session_id)?`
2. Creates manifest: `codelet_git::create_session_manifest(&session_id, &project, worktree_path, base_commit)?`
3. Creates session with worktree info populated
