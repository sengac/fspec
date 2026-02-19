# GIT-020: Isolated Session File Operations

## Overview

This story ensures that file operations in isolated sessions correctly use `effective_cwd()` for proper filesystem isolation. Files written by an isolated session should appear only in the worktree, not in the main project.

## Problem Statement

After GIT-019 implements `effective_cwd()`, the file operation tools (Read, Write, Edit, Bash) need to use it so that isolated sessions work in their worktree directory, not the main project.

## Solution

Ensure all file operations resolve paths relative to `effective_cwd()`:

1. Tool wrappers get session's effective_cwd
2. Relative paths resolved against effective_cwd
3. Bash commands run with cwd set to effective_cwd
4. Multiple isolated sessions can run in parallel without conflicts

## Scenarios Covered

| Scenario | Description |
|----------|-------------|
| File written by isolated session appears in worktree only | Write tool uses effective_cwd |
| File read by isolated session comes from worktree | Read tool uses effective_cwd |
| File operations use effective_cwd | All file tools respect isolation |
| Two isolated sessions run in parallel without conflict | Each session has its own worktree |
| Parallel sessions can modify same file independently | No file conflicts between sessions |

## Implementation Location

### Primary Files to Modify

```
codelet/tools/src/write.rs
├── Get session's effective_cwd from session context
└── Resolve paths relative to effective_cwd

codelet/tools/src/read.rs
├── Get session's effective_cwd from session context
└── Resolve paths relative to effective_cwd

codelet/tools/src/edit.rs
├── Get session's effective_cwd from session context
└── Resolve paths relative to effective_cwd

codelet/tools/src/bash.rs
├── Set working directory to effective_cwd
└── Execute commands in session's worktree
```

### Session Context Integration

```
codelet/tools/src/facade/wrapper.rs
├── Access current session via thread-local or parameter
└── Call session.effective_cwd() for path resolution
```

## API Design

### Path Resolution Pattern

```rust
impl WriteTool {
    async fn execute(&self, args: WriteArgs) -> Result<String> {
        // Get effective_cwd from current session
        let session = get_current_session()?;
        let effective_cwd = session.effective_cwd();
        
        // Resolve path relative to effective_cwd
        let resolved_path = if args.file_path.is_absolute() {
            args.file_path
        } else {
            effective_cwd.join(&args.file_path)
        };
        
        // Write to resolved path
        fs::write(&resolved_path, &args.content)?;
        Ok(format!("Written to {}", resolved_path.display()))
    }
}
```

### Bash Tool Integration

```rust
impl BashTool {
    async fn execute(&self, args: BashArgs) -> Result<String> {
        let session = get_current_session()?;
        let effective_cwd = session.effective_cwd();
        
        Command::new("bash")
            .arg("-c")
            .arg(&args.command)
            .current_dir(&effective_cwd)  // Run in session's worktree
            .output()
    }
}
```

## Test Strategy

Tests in `codelet/tools/tests/isolated_file_operations_test.rs`:

1. **Write isolation**: File written in isolated session only exists in worktree
2. **Read isolation**: File read comes from worktree, not main project
3. **Bash isolation**: `pwd` returns worktree path
4. **Parallel sessions**: Two sessions can modify same filename independently
5. **Main project unchanged**: Operations don't affect main project files

## Dependencies

- **GIT-019** (required): Provides `effective_cwd()` method

## Downstream Dependencies

None - this is a leaf story for file operation isolation.

## Acceptance Criteria Checklist

- [ ] Write tool creates files in worktree for isolated sessions
- [ ] Read tool reads files from worktree for isolated sessions
- [ ] Edit tool modifies files in worktree for isolated sessions
- [ ] Bash tool runs commands with cwd set to worktree
- [ ] Main project files unchanged by isolated session operations
- [ ] Parallel isolated sessions don't conflict
- [ ] Non-isolated sessions still work (backward compatible)
- [ ] All tests pass

---

## Next Steps

GIT-020 is a **leaf story** with no downstream dependencies. Once complete:

| Action | Description |
|--------|-------------|
| **Mark Done** | Move GIT-020 to `done` status |
| **Verify Integration** | File operations in isolated sessions work end-to-end |
| **Continue Parallel Work** | GIT-021 and GIT-022 can proceed independently |

## Story Dependency Graph

```
GIT-019 (Isolated Session Creation)
    │
    ├── GIT-020 (This Story) ◀── LEAF STORY (no downstream)
    │
    ├── GIT-021 (Checkpoints)
    │
    └── GIT-022 (Status Derivation)
            │
            └── GIT-023 (List/Inspect)
                    ...
```

## Related Stories

| Story | Relationship | Notes |
|-------|--------------|-------|
| GIT-019 | **Depends On** | Provides `effective_cwd()` method |
| GIT-021 | Parallel | Works on checkpoints, can proceed independently |
| GIT-022 | Parallel | Works on status derivation, can proceed independently |
