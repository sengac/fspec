# AST Research: KGRAPH-014 Parent Card — AST Connection Graph

## Overview

This is a **parent card** that organizes four child work units. All implementation, testing, and research was done in the children:

- **KGRAPH-016** — AST Graph Data Model & Schema (done)
- **KGRAPH-017** — AST Extraction Pipeline (done)
- **KGRAPH-018** — AST Dependency Graph Population (done)
- **KGRAPH-019** — AST Graph Query Interface (done)

## Implementation Files (from children)

### Data Model (KGRAPH-016)
- `codelet/napi/src/graph/database.rs` — GraphDatabase abstraction (268 lines)
- `codelet/napi/src/graph/registry.rs` — Named instance management (144 lines)
- `codelet/napi/src/graph/graph_entities.rs` — Shared entity types (66 lines)
- `codelet/napi/schemas/ast-code.pg` — PG schema (92 lines)

### Extraction Pipeline (KGRAPH-017)
- `codelet/napi/src/graph/ast_pipeline/mod.rs` — Directory walker (117 lines)
- `codelet/napi/src/graph/ast_pipeline/ast_ts_extractor.rs` — TypeScript (214 lines)
- `codelet/napi/src/graph/ast_pipeline/ast_rust_extractor.rs` — Rust (167 lines)
- `codelet/napi/src/graph/ast_pipeline/helpers.rs` — Shared helpers (187 lines)

### Dependencies (KGRAPH-018)
- `codelet/napi/src/graph/ast_pipeline/npm_dep_extractor.rs` — NPM (58 lines)
- `codelet/napi/src/graph/ast_pipeline/cargo_dep_extractor.rs` — Cargo (155 lines)

### Query Interface (KGRAPH-019)
- `codelet/napi/src/graph/ast_dispatch.rs` — AST dispatch (159 lines)
- `codelet/napi/schemas/ast-queries.gq` — Named queries

## Tests
All 15 AST-related tests pass across 4 test files:
- 5 data model tests
- 4 extraction pipeline tests
- 3 dependency population tests
- 3 query interface tests
