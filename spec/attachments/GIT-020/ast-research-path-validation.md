# AST Research: Isolated Session Path Validation

## Research Summary

This research analyzes the path validation mechanism for isolated sessions to ensure E2E tests properly verify the blocking behavior.

## Core Path Validation Functions

### 1. validate_and_resolve_path (Public API)
**Location:** `codelet/tools/src/facade/wrapper.rs:551`
```
pub fn validate_and_resolve_path(session_id, path, tool_name) -> Result<PathBuf, ToolError>
```
- Entry point for all tool path validation
- Calls `get_effective_cwd(session_id)` to get worktree path
- Delegates to `validate_and_resolve_path_with_cwd()`

### 2. validate_and_resolve_path_with_cwd (Internal Implementation)
**Location:** `codelet/tools/src/facade/wrapper.rs:569`
```
pub fn validate_and_resolve_path_with_cwd(path, worktree_path, tool_name) -> Result<PathBuf, ToolError>
```
- Core validation logic
- For isolated sessions (worktree_path is Some):
  - Relative paths: resolved to worktree → ALLOWED
  - Absolute paths within worktree → ALLOWED
  - Absolute paths outside worktree → BLOCKED with ToolError::Validation

### 3. get_effective_cwd (Callback Bridge)
**Location:** `codelet/tools/src/facade/wrapper.rs:509`
```
pub fn get_effective_cwd(session_id: Uuid) -> Option<PathBuf>
```
- Uses GET_EFFECTIVE_CWD_CALLBACK to call into NAPI layer
- Returns worktree path for isolated sessions, project root for non-isolated

### 4. get_session_effective_cwd (NAPI Callback)
**Location:** `codelet/napi/src/session_manager.rs:5950`
```
fn get_session_effective_cwd(session_id_str: String) -> Option<PathBuf>
```
- Registered during init_block_notification_callbacks()
- Looks up session from SessionManager
- Returns session.effective_cwd()

## Tools Using Path Validation

| Tool | File | Line | Tool Name |
|------|------|------|-----------|
| Read | wrapper.rs | 287 | "read" |
| Write | wrapper.rs | 320 | "write" |
| Edit | wrapper.rs | 370 | "edit" |
| Grep (with path) | wrapper.rs | 966 | "grep" |
| Grep (default) | wrapper.rs | 978 | "grep" |
| Glob (with path) | wrapper.rs | 1012 | "glob" |
| Glob (default) | wrapper.rs | 1024 | "glob" |
| Ls (with path) | wrapper.rs | 1147 | "ls" |
| Ls (default) | wrapper.rs | 1159 | "ls" |
| AstGrep (with path) | astgrep.rs | 393 | "ast_grep" |
| AstGrep (default) | astgrep.rs | 399 | "ast_grep" |
| AstGrepRefactor (source) | astgrep_refactor.rs | 1060 | "ast_grep_refactor" |
| AstGrepRefactor (target) | astgrep_refactor.rs | 1067 | "ast_grep_refactor" |

## Callback Registration Chain

```
sessionSetGlobalChunkCallback() (TypeScript startup)
    └── init_block_notification_callbacks() (session_manager.rs)
        └── set_get_effective_cwd_callback(get_session_effective_cwd)
            └── Registers callback in GET_EFFECTIVE_CWD_CALLBACK (wrapper.rs)
```

## E2E Test Requirements

For proper E2E testing, we need to:

1. **Create real isolated session** via `sessionManagerCreateIsolated` NAPI binding
2. **Ensure callback is registered** by calling `sessionSetGlobalChunkCallback` first
3. **Invoke path validation** via `sessionValidatePath` NAPI binding (GIT-020)
4. **Verify blocking** for paths outside worktree
5. **Verify allowing** for paths inside worktree

## New NAPI Bindings for E2E Testing (Added in GIT-020)

| Function | Purpose |
|----------|---------|
| sessionValidatePath | Validate if path is allowed for session |
| sessionGetEffectiveCwd | Get worktree/project path for session |
| sessionIsIsolated | Check if session has worktree |

## Test Scenarios

### BLOCKING (must return error):
- Absolute path to main project: `/project/src/main.ts`
- Path traversal: `../../src/main.ts`
- Symlink pointing outside worktree

### ALLOWED (must succeed):
- Relative path: `src/app.ts` → resolves to worktree
- Absolute path within worktree: `/project/.fspec/worktrees/xyz/src/app.ts`

### BACKWARD COMPATIBLE:
- Non-isolated session: all paths allowed
