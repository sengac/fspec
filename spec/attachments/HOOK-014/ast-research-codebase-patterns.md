# AST Research: Codebase Patterns for HOOK-014

## 1. Two-Level Config Loading Pattern (Blocklist Reference)

The existing blocklist module (`codelet/tools/src/blocklist/`) implements the same two-level hierarchy needed for lifecycle hooks:

- **System (user-level):** `~/.fspec/blocklist.json` via `dirs::home_dir()`
- **Project-level:** `<project_root>/.fspec/blocklist.json`
- **Merge:** Project rules prepended before system rules (first-match-wins for blocklist)
- **Hot-reload:** Blocklist reloads every check — lifecycle hooks will NOT hot-reload (rule: compiled once at session creation)

**For lifecycle hooks, paths are:**
- User-level: `~/.fspec/fspec-hooks.json`
- Project-level: `spec/fspec-hooks.json`
- Merge: user-level hooks first, project-level appended (concatenation, not override)

## 2. BackgroundSession Fields

`BackgroundSession` (session_manager.rs:468) has ~35 fields. The lifecycle hook engine should be added as:
```rust
lifecycle_hooks: Option<Arc<CompiledLifecycleHooks>>,
```

This follows the pattern of other optional session-scoped config:
- `worktree_path: Option<PathBuf>` (GIT-019)
- `work_unit_context: RwLock<Option<WorkUnitContext>>` (TUI-059)

## 3. Session Creation Flow

`create_session_with_id()` (lines 3196-3345) creates the session and spawns `agent_loop()`. The lifecycle hook engine should be:
1. Loaded and compiled during `create_session_with_id()`
2. Stored on `BackgroundSession`
3. Accessed by the agent loop via `session.lifecycle_hooks`

## 4. Module Registration Pattern

In `codelet/tools/src/lib.rs`, modules follow:
```rust
pub mod lifecycle_hooks;  // Module declaration
pub use lifecycle_hooks::{  // Re-exports
    CompiledLifecycleHooks, HookMatcher, load_lifecycle_hooks_config, ...
};
```

## 5. Workspace Dependencies Available

- `regex = "1"` — already in workspace deps
- `serde`, `serde_json` — already in codelet-core deps  
- `tokio` — already in codelet-core deps
- `dirs` — NOT in codelet-core deps (used by blocklist in codelet-tools). Need to check if we load config in codelet-core or codelet-tools or codelet-napi.

## 6. Recommended Module Location

Given that:
- codelet-core has `serde_json`, `regex`, and the agent execution context
- codelet-tools has `blocklist` (config loading pattern) but is tool-focused
- codelet-napi has `session_manager.rs` (where config needs to be loaded and stored)

**Decision: Place lifecycle hook config in codelet-core** since:
- It's not a tool — it's an execution engine that wraps tool calls
- codelet-core already has regex and serde_json dependencies
- The engine will be threaded through BackgroundSession which imports from codelet-core
- Need to add `dirs` dependency for home directory resolution

## 7. Key Files to Create/Modify

### New files in codelet-core:
- `src/lifecycle_hooks/mod.rs` — module root
- `src/lifecycle_hooks/config.rs` — serde types for fspec-hooks.json
- `src/lifecycle_hooks/compiled.rs` — compiled types with regex matchers
- `src/lifecycle_hooks/loader.rs` — two-level config loading and merging

### Modifications:
- `src/lib.rs` — add `pub mod lifecycle_hooks;` and re-exports
- `Cargo.toml` — add `dirs` dependency if not present
