# AST Research: Session ID for All Tools

## Summary

This document captures AST-based research for TOOL-014: "Require session_id for all tools to support worktree isolation".

## Current Tool State

### Tools That ALREADY Have session_id

| Tool | File | Constructor |
|------|------|-------------|
| BashTool | bash.rs:486 | `pub fn new(session_id: uuid::Uuid) -> Self` |
| BridgeManager | bridge.rs:126 | `pub fn new(session_id: Uuid) -> Self` |

### Tools That NEED session_id Added

| Tool | File | Current Constructor | Has Default |
|------|------|---------------------|-------------|
| ReadTool | read.rs:39 | `pub fn new() -> Self { Self }` | Yes (line 90) |
| WriteTool | write.rs:18 | `pub fn new() -> Self { Self }` | Yes (line 23) |
| EditTool | edit.rs:20 | `pub fn new() -> Self { Self }` | Yes (line 25) |
| GrepTool | grep.rs:52 | `pub fn new() -> Self { Self }` | Yes (line 332) |
| GlobTool | glob.rs:25 | `pub fn new() -> Self { Self }` | Yes (line 38) |
| LsTool | ls.rs:38 | `pub fn new() -> Self { Self }` | Yes (line 103) |
| AstGrepTool | astgrep.rs:26 | `pub fn new() -> Self { Self }` | Yes (line 319) |
| AstGrepRefactorTool | astgrep_refactor.rs:115 | `pub fn new() -> Self { Self }` | Yes (line 915) |
| WebSearchTool | web_search.rs:329 | `pub fn new() -> Self { Self }` | Yes (line 321) |
| FspecTool | fspec.rs:15 | `pub fn new() -> Self { Self {} }` | Yes (line 38) |
| BridgeTool | bridge.rs:225 | `pub fn new() -> Self { Self }` | Yes (line 217) |

## Facade Wrapper State

### Wrappers That ALREADY Handle session_id

| Wrapper | File | Pattern |
|---------|------|---------|
| FileToolFacadeWrapper | wrapper.rs:221 | `new(facade, session_id)` - passes session_id, calls `get_effective_cwd(self.session_id)` |
| BashToolFacadeWrapper | wrapper.rs:663 | `new(facade, session_id)` - creates `BashTool::new(session_id)` |
| FspecToolFacadeWrapper | wrapper.rs:555 | `new(facade, session_id)` - uses session_id for handler lookup |
| BridgeToolFacadeWrapper | wrapper.rs:992 | `new(facade, session_id)` - uses session_id for handler lookup |

### Wrappers That NEED session_id Added

| Wrapper | File | Current Constructor |
|---------|------|---------------------|
| SearchToolFacadeWrapper | wrapper.rs:773 | `new(facade)` - NO session_id, creates GrepTool/GlobTool without it |
| LsToolFacadeWrapper | wrapper.rs:885 | `new(facade)` - NO session_id, creates LsTool without it |
| FacadeToolWrapper | wrapper.rs:38 | `new(facade)` - creates WebSearchTool without session_id |

## Existing Helper Functions

### get_effective_cwd (wrapper.rs:469)
```rust
pub fn get_effective_cwd(session_id: Uuid) -> Option<PathBuf> {
    GET_EFFECTIVE_CWD_CALLBACK.get()
        .and_then(|callback| callback(session_id.to_string()))
}
```

### resolve_file_path (wrapper.rs:499)
```rust
fn resolve_file_path(file_path: &str, effective_cwd: Option<&PathBuf>) -> String {
    let path = std::path::Path::new(file_path);
    if path.is_absolute() {
        return file_path.to_string();
    }
    if let Some(cwd) = effective_cwd {
        return cwd.join(file_path).to_string_lossy().to_string();
    }
    file_path.to_string()
}
```

**Note:** The current `resolve_file_path` does NOT validate that absolute paths are within the worktree. This is a gap identified by TOOL-014.

## Implementation Pattern (from BashTool)

The BashTool implementation (bash.rs) shows the correct pattern:

```rust
pub struct BashTool {
    session_id: uuid::Uuid,
}

impl BashTool {
    pub fn new(session_id: uuid::Uuid) -> Self {
        Self { session_id }
    }

    fn get_effective_cwd(&self) -> Option<std::path::PathBuf> {
        crate::facade::get_effective_cwd(self.session_id)
    }
}
```

## Required Changes

### 1. Tool Struct Changes

For each tool (ReadTool, WriteTool, EditTool, GrepTool, GlobTool, LsTool, AstGrepTool, AstGrepRefactorTool, WebSearchTool):

```rust
pub struct XxxTool {
    session_id: uuid::Uuid,
}

impl XxxTool {
    pub fn new(session_id: uuid::Uuid) -> Self {
        Self { session_id }
    }

    fn get_effective_cwd(&self) -> Option<std::path::PathBuf> {
        crate::facade::get_effective_cwd(self.session_id)
    }
}
```

### 2. Remove Default Implementations

All `impl Default for XxxTool` must be removed since session_id cannot have a default value.

### 3. Path Validation Helper

Create a new helper function to validate paths are within worktree:

```rust
/// Validate and resolve a path for worktree isolation.
/// 
/// If session has an effective_cwd (worktree):
/// - Relative paths are resolved relative to worktree
/// - Absolute paths outside worktree are rejected
/// 
/// If session has no effective_cwd:
/// - Paths are returned as-is (normal operation)
fn validate_and_resolve_path(
    session_id: Uuid,
    path: &str,
    tool_name: &'static str,
) -> Result<PathBuf, ToolError> {
    let effective_cwd = get_effective_cwd(session_id);
    
    match effective_cwd {
        Some(worktree_path) => {
            let path_buf = std::path::Path::new(path);
            
            if path_buf.is_absolute() {
                // Check if absolute path is within worktree
                let canonical_worktree = worktree_path.canonicalize()
                    .map_err(|e| ToolError::File {
                        tool: tool_name,
                        message: format!("Cannot canonicalize worktree: {e}"),
                    })?;
                let canonical_path = path_buf.canonicalize()
                    .map_err(|e| ToolError::File {
                        tool: tool_name,
                        message: format!("Cannot canonicalize path: {e}"),
                    })?;
                
                if !canonical_path.starts_with(&canonical_worktree) {
                    return Err(ToolError::Validation {
                        tool: tool_name,
                        message: format!(
                            "Path is outside isolated worktree. Use relative path or path within worktree."
                        ),
                    });
                }
                
                Ok(canonical_path)
            } else {
                // Relative path - resolve to worktree
                Ok(worktree_path.join(path))
            }
        }
        None => {
            // No worktree isolation - normal operation
            Ok(PathBuf::from(path))
        }
    }
}
```

### 4. Update Wrapper Constructors

```rust
// SearchToolFacadeWrapper
impl SearchToolFacadeWrapper {
    pub fn new(facade: BoxedSearchToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            grep_tool: GrepTool::new(session_id),
            glob_tool: GlobTool::new(session_id),
            session_id,
        }
    }
}

// LsToolFacadeWrapper
impl LsToolFacadeWrapper {
    pub fn new(facade: BoxedLsToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            ls_tool: LsTool::new(session_id),
            session_id,
        }
    }
}
```

### 5. Update FileToolFacadeWrapper

Currently creates tools with `::new()` - needs to pass session_id:

```rust
impl FileToolFacadeWrapper {
    pub fn new(facade: BoxedFileToolFacade, session_id: Uuid) -> Self {
        Self {
            facade,
            read_tool: ReadTool::new(session_id),
            write_tool: WriteTool::new(session_id),
            edit_tool: EditTool::new(session_id),
            session_id,
        }
    }
}
```

## Test Pattern

For tests that don't need worktree isolation, use `Uuid::nil()`:

```rust
#[test]
fn test_read_tool_reads_file() {
    let tool = ReadTool::new(Uuid::nil());
    // ... test code
}
```

## Affected Files Summary

| File | Changes |
|------|---------|
| read.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| write.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| edit.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| grep.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| glob.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| ls.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| astgrep.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| astgrep_refactor.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default, validate paths in call() |
| web_search.rs | Add session_id field, new(session_id), get_effective_cwd(), remove Default (WebSearchTool doesn't do file ops, but should have session_id for consistency) |
| fspec.rs | Already handled by FspecToolFacadeWrapper - no changes needed |
| bridge.rs | BridgeTool already handled by BridgeToolFacadeWrapper - no changes needed |
| facade/wrapper.rs | Update SearchToolFacadeWrapper and LsToolFacadeWrapper to take and pass session_id; Update FileToolFacadeWrapper to pass session_id to tool constructors; Add validate_and_resolve_path helper |
| lib.rs | May need to update re-exports if tool construction changes |
