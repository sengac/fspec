# AST Research: Tool Facade Wrapper Architecture

## Research Summary

Analyzed `codelet/tools/src/facade/wrapper.rs` to understand the existing tool execution architecture.

## Key Findings

### Tool Wrapper Types

| Wrapper | Base Tool | Intercept Point |
|---------|-----------|-----------------|
| `BashToolFacadeWrapper` | `BashTool` | `call()` method line 497-519 |
| `FileToolFacadeWrapper` | `ReadTool`, `WriteTool`, `EditTool` | `call()` method line 243-315 |
| `SearchToolFacadeWrapper` | `GrepTool`, `GlobTool` | `call()` method line 588-630 |
| `LsToolFacadeWrapper` | `LsTool` | `call()` method line 699-721 |
| `FspecToolFacadeWrapper` | TypeScript handler | `call()` method line 404-447 |
| `BridgeToolFacadeWrapper` | Bridge handler | `call()` method line 807-841 |

### BashToolFacadeWrapper Integration Point

```rust
// codelet/tools/src/facade/wrapper.rs:497-519
async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
    // Use the facade to map provider-specific params to internal format
    let internal_params = self.facade.map_params(args.0)?;

    // Execute the bash tool based on the operation type
    match internal_params {
        InternalBashParams::Execute { command } => {
            // *** BLOCKLIST CHECK GOES HERE ***
            // Before executing, check if command matches any blocklist rules
            let bash_args = BashArgs { command };
            match self.bash_tool.call(bash_args).await {
                Ok(output) => Ok(BashOperationResult { ... }),
                Err(e) => Ok(BashOperationResult { ... }),
            }
        }
    }
}
```

### FileToolFacadeWrapper Integration Point (for Write operations)

```rust
// codelet/tools/src/facade/wrapper.rs:274-288
InternalFileParams::Write { file_path, content } => {
    // *** STAGE PERMISSIONS CHECK GOES HERE (BLOCK-003) ***
    // Before writing, check if file_path is writable in current stage
    use crate::write::WriteArgs;
    let write_args = WriteArgs { file_path, content };
    match self.write_tool.call(write_args).await {
        ...
    }
}
```

## Architecture Decision

### Option A: Middleware Pattern (Recommended)
Create `BlocklistMiddleware` that wraps tool execution:
```rust
pub struct BlocklistMiddleware {
    config: BlocklistConfig,
    session_allowances: Arc<RwLock<HashSet<String>>>,
}

impl BlocklistMiddleware {
    pub fn check_command(&self, command: &str) -> Result<(), BlockedError>;
    pub fn check_file_write(&self, path: &str, stage: &str) -> Result<(), BlockedError>;
}
```

### Option B: Direct Integration
Modify each wrapper's `call()` method to check blocklist before execution.

**Recommendation:** Option A (Middleware Pattern) for cleaner separation of concerns.

## Integration Points Summary

1. **BashToolFacadeWrapper::call()** - Check command against blocklist rules
2. **FileToolFacadeWrapper::call()** for Write/Edit - Check path against stage permissions (BLOCK-003)
3. **Config Loading** - Load from `~/.fspec/blocklist.json` and `.fspec/blocklist.json`

## Files to Create

```
codelet/tools/src/blocklist/
├── mod.rs           # Module exports
├── config.rs        # BlocklistConfig, BlocklistRule
├── matcher.rs       # BlocklistMatcher (regex evaluation)
└── middleware.rs    # BlocklistMiddleware (intercepts tool execution)
```

## NAPI Bindings Required

- `blocklist_load(project_root: String) -> BlocklistConfig`
- `blocklist_save(project_root: String, config: BlocklistConfig)`
- `blocklist_check(command: String) -> CheckResult`
- `blocklist_allow_session(pattern: String)` - For BLOCK-005
