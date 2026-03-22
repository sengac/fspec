# AST Research: KGRAPH-015 Parent Card — Learnings Graph

## Overview

This is a **parent card** that organizes four child work units for the Learnings Graph. All implementation, testing, and research was done in the children:

- **KGRAPH-020** — Learnings Graph Data Model & Schema (done)
- **KGRAPH-021** — Learnings Extraction Pipeline (done)
- **KGRAPH-022** — Cross-Session Learning & Context Injection (done)
- **KGRAPH-023** — Learnings Graph Query Interface (done)

## Implementation Files (from children)

### Data Model (KGRAPH-020)
- `codelet/napi/schemas/learnings.pg` — PG schema (112 lines)
- `codelet/napi/src/graph/database.rs` — Shared GraphDatabase (268 lines)
- `codelet/napi/src/graph/registry.rs` — Instance registry (144 lines)

### Extraction Pipeline (KGRAPH-021)
- `codelet/napi/src/graph/learnings_extraction.rs` — Residue methodology extraction (270 lines)
- `codelet/napi/src/graph/llm_response_parser.rs` — Shared JSON parsing (72 lines)

### Context Injection (KGRAPH-022)
- `codelet/napi/src/graph/learnings_context.rs` — Session context builder (236 lines)

### Query Interface (KGRAPH-023)
- `codelet/napi/src/graph/learnings_dispatch.rs` — Query dispatch (214 lines)
- `codelet/napi/src/graph/dispatch_helpers.rs` — Shared helpers (74 lines)
- `codelet/napi/schemas/learnings-queries.gq` — Named queries

## Tests
All 17 Learnings-related tests pass across 4 test files:
- 4 data model tests
- 3 extraction pipeline tests
- 6 cross-session learning tests
- 4 query interface tests
