# Implementation Roadmap — Child Work Units

## Proposed Breakdown

### Phase 1: Core Infrastructure
1. **KGRAPH-002: Nanograph Integration & Database Lifecycle**
   - Add `nanograph-db` npm dependency (or compile from source as workspace crate)
   - Database init/open/close lifecycle in the Rust NAPI layer
   - Schema file management (agent-memory.pg bundled with fspec)
   - `~/.fspec/graph/` directory management
   - Estimate: 8 points

2. **KGRAPH-003: GraphSearch Tool Definition & Handler Registration**
   - Tool schema (actions, parameters, JSON schema)
   - Handler registry pattern (same as SessionSearch)
   - NanographBridge wrapper in Rust
   - Register in tool set
   - Estimate: 5 points

### Phase 2: Graph Population
3. **KGRAPH-004: Structural Extractors (Zero-Cost)**
   - Extract from fspec tool calls → WorkUnit nodes
   - Extract from Write/Edit tool calls → CodeEntity nodes + Modifies edges
   - Extract from error→success patterns → lightweight Decision detection
   - Wire into message persistence as optional hook
   - Estimate: 5 points

4. **KGRAPH-005: LLM-Based Concept Extraction Pipeline**
   - Extraction prompt design and tuning
   - Batch processing (5-10 turns per call)
   - JSON response parsing and validation
   - Concept/Decision/Relation/CodeEntity extraction
   - Estimate: 8 points

5. **KGRAPH-006: Graph Merge & Upsert Logic**
   - JSONL generation for nanograph load
   - Merge mode for idempotent upserts
   - Co-occurrence counting for RelatesTo edges
   - Watermark tracking (index-state.json)
   - Estimate: 5 points

### Phase 3: Search & Query
6. **KGRAPH-007: GraphSearch Action Implementations**
   - `search` — hybrid semantic + text concept search
   - `neighbors` — bounded expansion queries
   - `path` — shortest path between concepts
   - `related` — RelatesTo edge traversal
   - `decisions` — decision history queries
   - `history` — concept timeline
   - `stats` — graph statistics
   - Estimate: 8 points

### Phase 4: Automation
7. **KGRAPH-008: Scheduled Indexing Cron Job**
   - Skills file schema and validation
   - Schedule tool integration
   - Incremental watermark-based indexing
   - Index scope filtering (projects, sessions)
   - Estimate: 5 points

8. **KGRAPH-009: DeepSearch Graph Integration**
   - Add GraphSearch as optional DeepSearch sub-agent tool
   - `update_graph` flag for piggyback indexing
   - Graph context injection into DeepSearch system prompt
   - Estimate: 5 points

### Phase 5: Maintenance & Polish
9. **KGRAPH-010: Graph Compaction & Pruning**
   - Turn node pruning (configurable retention)
   - Similar concept merging
   - Lance storage compaction scheduling
   - Estimate: 3 points

10. **KGRAPH-011: Schema Migration Support**
    - Versioned schema files
    - Automatic migration on open
    - Backward compatibility
    - Estimate: 3 points

## Total Estimate: ~55 points

## Dependency Chain
```
KGRAPH-002 (infra) 
    → KGRAPH-003 (tool) 
        → KGRAPH-007 (queries)
    → KGRAPH-004 (structural extract)
    → KGRAPH-005 (LLM extract) → KGRAPH-006 (merge)
        → KGRAPH-008 (cron)
        → KGRAPH-009 (deepsearch)
    → KGRAPH-010 (compaction)
    → KGRAPH-011 (migration)
```

## Open Questions for Discovery

1. **Embedding provider**: Should we use the user's configured LLM provider for embeddings, or always OpenAI? Nanograph's `@embed` expects OpenAI-compatible API.
2. **Cross-project graphs**: ~~Start project-local only, or global from day one?~~ **DECIDED: Global from day one. Graph lives in `~/.fspec/graph/` alongside sessions/messages.**
3. **Extraction model**: Should the extraction LLM be configurable, or hardcoded to a fast/cheap model?
4. **Existing GRAPH-001/002/003**: The backlog has existing `GRAPH-*` work units for a different graph system (ngraph.graph for command navigation). Should we supersede those or keep them separate?
5. **nanograph-ts build**: Do we vendor nanograph as a git submodule and build from source, or publish a pre-built npm package?
