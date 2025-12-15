# AST Research: Current Module Structure for REFAC-002

## Analysis Date
2025-12-02

## Purpose
Analyze current src/lib.rs module structure to understand where rig-core re-exports should be added.

## Current src/lib.rs Structure

### Module Declarations
```rust
pub mod agent;
pub mod cli;
pub mod context;
pub mod providers;
pub mod tools;
```

### Documentation
The file includes comprehensive documentation describing the 5 bounded contexts:
- **cli**: CLI Interface - Command parsing, configuration, entry points
- **providers**: Provider Management - Multi-provider LLM abstraction
- **tools**: Tool Execution - File operations, code search, bash execution
- **context**: Context Management - Token tracking, compaction, prompt caching
- **agent**: Agent Execution - Main runner loop, message history, streaming

## Current Dependencies (Cargo.toml Analysis Required)
Need to analyze Cargo.toml to verify current dependency list before adding rig-core 0.25.0.

## Implementation Plan for REFAC-002

### Step 1: Add rig-core Dependency
Add to Cargo.toml dependencies section:
```toml
rig-core = "0.25.0"
```

### Step 2: Re-export rig Types
Add to src/lib.rs (after module declarations, before or after existing modules):
```rust
// Re-export rig types for internal use and future refactoring
pub use rig;
```

This will make rig modules accessible via `codelet::rig::*` namespace.

### Key Points
1. **No Breaking Changes**: Existing public API remains unchanged
2. **Additive Only**: Only adding dependency and re-export
3. **Foundation for Future**: Enables REFAC-003 (provider refactor) and REFAC-004 (agent refactor)
4. **Backwards Compatible**: All existing code continues to work without modification

## Verification Steps
1. `cargo build` - Must succeed without errors
2. `cargo test` - All existing tests must pass
3. `cargo clippy -- -D warnings` - Must complete with zero warnings
4. Test re-export accessibility: Can import `use codelet::rig::completion::CompletionModel;` in test files

## References
- rig-core documentation: https://docs.rs/rig-core/0.25.0
- REFAC-001 research: spec/attachments/REFAC-001/rig-refactoring-research.md
