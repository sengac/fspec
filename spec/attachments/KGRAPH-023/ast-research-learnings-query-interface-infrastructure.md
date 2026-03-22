# AST Research: Learnings Graph Query Interface Infrastructure

## Research Date: 2026-03-22

## Purpose
Researched existing GraphSearch tool infrastructure to understand the pattern for adding Learnings-specific query actions alongside existing agent-memory and AST query actions.

## Architecture Analysis

### Three-Layer Architecture
```
Layer 1: Tool Definition (codelet-tools/src/graph_search/)
  types.rs → GraphSearchAction enum (serde-tagged)
  handler.rs → Per-session handler registry (sync bridge)
  mod.rs → Rig Tool trait impl + JSON schema definition

Layer 2: Handler Factory (codelet-napi/src/graph_search_handler.rs)
  Creates Arc<closure> per session that bridges sync→async
  dispatch_action() — the central match statement

Layer 3: Dispatch Functions (codelet-napi/src/graph/)
  dispatch.rs → Agent-memory graph queries
  ast_dispatch.rs → AST code graph queries
  learnings_dispatch.rs → (TO CREATE) Learnings graph queries
```

### Existing Action Types (11 total)

**Agent-Memory Actions (8):** Search, Neighbors, Path, Related, Decisions, History, Stats, Index
**AST Code Actions (3):** AstSearch, AstNeighbors, AstStats

### Pattern for Adding Learnings Actions

Following the exact same pattern as AST actions:

1. **types.rs** — Add enum variants: LearningsSearch, LearningsDecisions, LearningsStats, LearningsRelated
2. **learnings_dispatch.rs** — New file with dispatch functions, each taking `&GraphDatabase`
3. **learnings-queries.gq** — New query file for nanograph queries
4. **graph_search_handler.rs** — Add match arms routing via `registry::get_graph(LEARNINGS_GRAPH)`
5. **graph/mod.rs** — Add `pub mod learnings_dispatch;`

### Key Files to Modify
- `codelet/tools/src/graph_search/types.rs` (add 4 enum variants)
- `codelet/napi/src/graph_search_handler.rs` (add 4 match arms)
- `codelet/napi/src/graph/mod.rs` (add module export)

### Key Files to Create
- `codelet/napi/src/graph/learnings_dispatch.rs` (dispatch functions)
- `codelet/napi/schemas/learnings-queries.gq` (nanograph queries)

### Learnings Graph Schema (learnings.pg)
- **Node types:** Learning, Exploration, Convention, Decision, CodePattern
- **Edge types:** Discovered, Eliminates, Supersedes, RelatesTo, InformedBy, Applies, Contradicts
- **Learning categories:** convention, pattern, anti_pattern, decision, discovery, constraint, reformulation
- **Decision domains:** architecture, convention, dependency, deployment, design, implementation, process, testing
- **Decision statuses:** active, proposed, reversed, superseded

### AST Grep Results
- `dispatch_ast_search(db, query, entity_type, limit)` and `dispatch_ast_neighbors(db, node_id, depth, edge_types)` in `ast_dispatch.rs:47,103`
- `GraphSearchAction` enum in `types.rs:11` with 11 variants
- AST actions use `include_str!("../../schemas/ast-queries.gq")` for query source
