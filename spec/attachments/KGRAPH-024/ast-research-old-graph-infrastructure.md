# AST Research: Old Graph Infrastructure for Deprecation (KGRAPH-024)

## Summary

Research performed via DeepSearch exploring `codelet/napi/src/graph/`, `codelet/napi/schemas/`, and `codelet/tools/src/graph_search/` to identify all files related to the old monolithic graph that need deprecation.

## Files to Delete (18 files, ~4200 lines)

### Schemas (2 files, 214 lines)
- `codelet/napi/schemas/agent-memory.pg` (143 lines) — Old NanoGraph schema with Turn/Session/Mentions nodes
- `codelet/napi/schemas/graph-queries.gq` (71 lines) — Named queries for agent-memory graph

### Entity Pipeline (2 files, 599 lines)
- `codelet/napi/src/graph/entity_pipeline.rs` (157 lines) — Global PENDING_ENTITIES queue, per-tool-call interception
- `codelet/napi/src/graph/extractors.rs` (442 lines) — GraphEntity enum, EntityQueue, Turn/Session/CodeEntity creation

### LLM Extraction Pipeline (3 files, 1188 lines)
- `codelet/napi/src/graph/llm_extraction.rs` (346 lines) — LLM concept extraction prompt builder
- `codelet/napi/src/graph/llm_validation.rs` (318 lines) — LLM response validation with hard-coded enums
- `codelet/napi/src/graph/llm_caller.rs` (524 lines) — LLM extraction orchestrator with batching

### Merge/Watermark/Indexing (4 files, 1143 lines)
- `codelet/napi/src/graph/merge.rs` (558 lines) — JSONL conversion + merge/upsert logic
- `codelet/napi/src/graph/watermark.rs` (100 lines) — Incremental indexing watermark state
- `codelet/napi/src/graph/indexing.rs` (159 lines) — Skills file config parsing
- `codelet/napi/src/graph/session_scanner.rs` (326 lines) — Session scanning pipeline

### Compaction (1 file, 276 lines)
- `codelet/napi/src/graph/compaction.rs` (276 lines) — Turn node pruning + schema migration

### Old Query/Dispatch (2 files, 700 lines)
- `codelet/napi/src/graph/dispatch.rs` (357 lines) — Old dispatch_search/neighbors/related/decisions/history/index
- `codelet/napi/src/graph/queries.rs` (343 lines) — GraphQueryResult + formatters for agent-memory

### DeepSearch Integration (1 file, 150 lines)
- `codelet/napi/src/graph/deepsearch_integration.rs` (150 lines) — Old agent-memory context injection

### Tests (1 file)
- `codelet/napi/src/graph/tests.rs` (7 lines) — Stub pointing to graph_lifecycle_test.rs

## Files to Surgically Update (7 files, ~1200 lines)

### Shared Infrastructure
- `codelet/napi/src/graph/database.rs` (269 lines) — Keep: GraphDatabase struct. Remove: agent-memory references
- `codelet/napi/src/graph/registry.rs` (163 lines) — Remove: AGENT_MEMORY_GRAPH constant. Keep: AST_CODE_GRAPH, LEARNINGS_GRAPH
- `codelet/napi/src/graph/dispatch_helpers.rs` (56 lines) — Keep: shared by ast_dispatch and learnings_dispatch
- `codelet/napi/src/graph/llm_response_parser.rs` (73 lines) — Keep: shared JSON extraction utility
- `codelet/napi/src/graph/mod.rs` (128 lines) — Remove old module declarations and public API

### Tool Layer
- `codelet/tools/src/graph_search/types.rs` (99 lines) — Remove: old 8 action variants. Keep: AST/Learnings
- `codelet/tools/src/graph_search/mod.rs` (147 lines) — Update tool definition schema
- `codelet/tools/src/graph_search/handler.rs` (68 lines) — Remove old handler references

### Handler
- `codelet/napi/src/graph_search_handler.rs` — Remove old dispatch routing, keep AST/Learnings

## Key Old Patterns to Remove

1. **Global EntityQueue** — `entity_pipeline.rs` lazy_static PENDING_ENTITIES
2. **Per-tool-call interception** — extract_and_queue_from_tool_call()
3. **Session scanning pipeline** — watermark-based incremental indexing
4. **Turn/Session provenance model** — the root cause of 7.6GB disk usage
5. **LLM concept/decision/relation extraction** — replaced by Learnings extraction
6. **Old GraphSearch actions** — Search, Neighbors, Path, Related, Decisions, History, Stats, Index
