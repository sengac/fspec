# AST Research: Learnings Graph — Existing Graph Infrastructure Analysis

## Research Method
DeepSearch analysis of `codelet/napi/src/graph/registry.rs`, `codelet/napi/src/graph/database.rs`, `codelet/napi/src/graph/mod.rs`

## Architecture

### 3-Layer Pattern
```
mod.rs (convenience facade, hard-coded to agent-memory)
  → registry.rs (named singleton manager, match-dispatched)
    → database.rs (generic, reusable, no global state)
```

### Current Named Graphs
1. `"agent-memory"` — Global at `~/.fspec/graph/agent-memory.nano/`
2. `"ast-code"` — Project-scoped at `<project>/.fspec/graph/ast-code.nano/`

### Changes Required for "learnings" Graph

**registry.rs** (~10 lines):
1. Add `pub const LEARNINGS_GRAPH: &str = "learnings";`
2. Add `const LEARNINGS_SCHEMA: &str = include_str!("../../schemas/learnings.pg");`
3. Add match arm in `resolve_graph_config()` — global scope (`get_data_dir()`)
4. Update error message in `_` arm

**No changes needed in database.rs** — already fully generic.

### Key Design Decision
Learnings graph is GLOBAL scope (shared across projects) — uses `get_data_dir()` not `resolve_project_dir()`.

### Schema Requirements
- Use `Bool` (not `Boolean`) for nanograph PG type compatibility
- All node types need `slug: String @key`
- Use `enum()` for categorical fields
