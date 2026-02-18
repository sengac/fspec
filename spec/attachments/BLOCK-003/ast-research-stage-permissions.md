# AST Research: Stage Permissions Module

## Overview

Research findings for implementing BLOCK-003: Stage Permissions - ACDD File Write Enforcement

## Existing Module Pattern: blocklist/

The blocklist module at `codelet/tools/src/blocklist/` provides the pattern to follow:

### File Structure
```
blocklist/
├── config.rs    # BlocklistConfig, BlocklistRule, BlocklistAction
├── matcher.rs   # BlocklistMatcher, CheckResult (regex evaluation)
├── middleware.rs # Global state, loading, checking functions
└── mod.rs       # Module exports and tests
```

### Key Types (from blocklist/config.rs)
- `BlocklistConfig`: JSON-serializable config with version and rules array
- `BlocklistRule`: Single rule with pattern, action, reason, guidance
- `BlocklistAction`: Enum (Block, Allow, Prompt)

### Middleware Pattern (from blocklist/middleware.rs)
- Global state via `RwLock<Option<Matcher>>`
- `init_blocklist(project_root)`: Initialize at startup
- `check_bash_command(command)`: Returns `Result<(), BlockedError>`
- Config loading from system (`~/.fspec/`) and project (`.fspec/`) paths
- Project config takes precedence over system config

## Work Unit Context

From `codelet/napi/src/session_manager.rs`:

```rust
pub struct WorkUnitContext {
    pub id: Option<String>,
    pub title: Option<String>,
    pub status: Option<String>,  // "backlog", "specifying", "testing", "implementing", "validating", "done"
}
```

Access via:
- `session.get_work_unit_context()` on Session struct
- `session_get_work_unit_context(session_id)` NAPI function

## Write Tool

From `codelet/tools/src/write.rs`:

Simple tool that:
1. Validates absolute path
2. Creates parent directories
3. Writes file contents

Integration point: Add stage permissions check before `write_file_contents()`

## Proposed Stage Permissions Module

### File Structure
```
stage_permissions/
├── config.rs    # StagePermissionsConfig, FileCategory, StagePermissions
├── matcher.rs   # Glob pattern matching for file categorization
└── mod.rs       # Module exports, loading, checking functions, tests
```

### Key Types (to create)

```rust
// File category: named group of glob patterns
pub struct FileCategory {
    pub name: String,           // e.g., "spec", "test", "impl"
    pub patterns: Vec<String>,  // e.g., ["spec/**/*.feature", "spec/**/*.md"]
}

// Stage permissions: which categories are writable in each stage
pub struct StagePermissions {
    pub stage: String,              // e.g., "testing"
    pub writable_categories: Vec<String>,  // e.g., ["spec", "test"]
}

// Full config
pub struct StagePermissionsConfig {
    pub version: String,
    pub categories: Vec<FileCategory>,
    pub permissions: Vec<StagePermissions>,
}
```

### Default Permissions
```
backlog     = spec
specifying  = spec
testing     = spec, test
implementing = spec, test, impl
validating  = nothing
done        = nothing
```

### Integration Points

1. **Write tool**: Add `check_write_permission(path, stage)` before writing
2. **Edit tool**: Same check
3. **Session context**: Get current stage from `session.get_work_unit_context().status`

## Config File Paths

- System: `~/.fspec/stage-permissions.json`
- Project: `.fspec/stage-permissions.json`
- Project takes precedence

## Example Config

```json
{
  "version": "1.0.0",
  "categories": [
    {
      "name": "spec",
      "patterns": ["spec/**/*.feature", "spec/**/*.md", "spec/attachments/**"]
    },
    {
      "name": "test",
      "patterns": ["src/**/*.test.ts", "src/**/__tests__/**", "**/*.spec.ts"]
    },
    {
      "name": "impl",
      "patterns": ["src/**/*.ts", "!src/**/*.test.ts", "!src/**/__tests__/**"]
    }
  ],
  "permissions": [
    { "stage": "backlog", "writable_categories": ["spec"] },
    { "stage": "specifying", "writable_categories": ["spec"] },
    { "stage": "testing", "writable_categories": ["spec", "test"] },
    { "stage": "implementing", "writable_categories": ["spec", "test", "impl"] },
    { "stage": "validating", "writable_categories": [] },
    { "stage": "done", "writable_categories": [] }
  ]
}
```

## Dependencies

- `globset` crate for glob pattern matching (already in project)
- `serde`, `serde_json` for config serialization
