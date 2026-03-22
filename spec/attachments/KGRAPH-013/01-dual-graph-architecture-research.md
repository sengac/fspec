# Dual-Graph Architecture Research

## Problem Statement

The original KGRAPH implementation (KGRAPH-002 through KGRAPH-012) indexed ALL conversation history into a single nanograph database. After 727 turns the database consumed **7.6GB** of disk space. This is unsustainable for several reasons:

### Root Causes of Disk Bloat

1. **Per-line JSONL loading anti-pattern**: `session_scanner.rs` loads each entity individually via `graph_db_load_jsonl(line)`, creating a new Lance dataset version per entity. A session with 100 entities creates 100 Lance versions per dataset type. This is the **primary amplification factor**.

2. **No Lance compaction/GC**: `compaction.rs` has Turn-pruning logic (`identify_turns_to_prune`) but nothing executes Lance `compact_files()` or `cleanup_old_versions()`. Historical versions accumulate indefinitely.

3. **Turn nodes as highest volume**: Every assistant message generates Turn + Mentions + Modifies edges. With 90-day default retention and no actual pruning, Turn is the biggest node type.

4. **Write amplification from MergeInsert**: Every `@key` upsert rewrites affected Lance fragments. Concepts with incrementally-updated `mentionCount` fields trigger repeated rewrites.

5. **Arrow columnar overhead**: Lance stores data in Arrow format with per-column metadata. High overhead for small-row-count datasets.

### Core Design Flaw

The fundamental issue is that the system tries to be a **comprehensive index of all conversation history** — which is a task better suited to SessionSearch. The graph should capture **structured knowledge**, not raw data. Two distinct use cases have been conflated:

1. **Code structure understanding** — "How does module X connect to module Y?", "What files implement this pattern?", "What are the call chains from this function?"
2. **Accumulated learnings** — "What approaches have been tried for problem X?", "What conventions have been established?", "What decisions were made and why?"

## Proposed Architecture: Two Purpose-Built Graphs

### Graph 1: AST Connection Graph
**Purpose**: Map the full codebase (and optionally dependency source) into a structural graph for code understanding queries.

**Key characteristics**:
- Built from **static analysis** (AST parsing via tree-sitter / ast-grep) — zero LLM cost
- Connects files → modules → functions → types → imports → call sites
- Can include dependency code (node_modules, crate dependencies) for full chain analysis
- Rebuilt on demand (not incrementally indexed from conversations)
- Answers: "What calls this function?", "What does this module depend on?", "Show me the type hierarchy"

**Storage**: Compact — a 100K LOC codebase produces ~10-50K nodes, well under 100MB even with Lance overhead.

See: `spec/attachments/KGRAPH-014/` for detailed AST graph research.

### Graph 2: Learnings Graph
**Purpose**: Capture accumulated knowledge, decisions, patterns, and failed approaches across agent sessions — following the "Residue" methodology from Aquino-Michaels' multi-agent structured exploration work.

**Key characteristics**:
- Built from **LLM extraction** on session-end summaries, not raw turn data
- Captures: explorations (what was tried), failures (structural reasons), surviving structure, reformulations, decisions, conventions
- Follows the "Strategy Register" pattern: eliminated approach classes, active constraints, known reformulations
- Cross-session synthesis: periodic synthesis entries that connect insights across sessions
- Answers: "What approaches have been tried for X?", "What conventions exist?", "What was decided about Y?"

**Storage**: Very compact — one session produces ~5-20 entities, not hundreds.

See: `spec/attachments/KGRAPH-019/` for detailed Learnings graph research.

## Existing Code Disposition

The current KGRAPH implementation (15 Rust files in `codelet/napi/src/graph/`) contains reusable infrastructure:

### Keep / Refactor
- `database.rs` — Graph lifecycle management (init, open, close, schema loading). Refactor to manage two graph instances.
- `merge.rs` — JSONL conversion, merge semantics. Useful for both graphs.
- `schemas/` — PG schema definitions. Need new schemas for both graphs.
- `dispatch.rs` — Query dispatch pattern. Extend for dual-graph routing.
- `watermark.rs` — Incremental indexing state. Only needed for Learnings graph.

### Replace
- `session_scanner.rs` — The per-line loading anti-pattern. Replace with batch loading.
- `extractors.rs` — Structural extractors tied to tool-call interception. Replace with AST-based extraction for Graph 1 and session-summary extraction for Graph 2.
- `entity_pipeline.rs` — Real-time entity queueing. May not be needed — both graphs can be built on-demand or at session boundaries.
- `llm_extraction.rs` — LLM extraction pipeline. Refactor for Learnings graph with Residue-style structured extraction.

### Remove
- Turn/Session/ContainsTurn node types — These replicate SessionSearch's job.
- Per-tool-call entity interception — Too granular; causes the volume problem.

## Migration Path

1. Create new KGRAPH prefix cards under KGRAPH-013
2. Implement AST graph first (zero LLM cost, immediate utility)
3. Implement Learnings graph second (requires extraction prompt redesign)
4. Deprecate old graph database and schema
5. Migrate any useful existing data (Concepts, Decisions) to Learnings graph

## References

- Current implementation: `codelet/napi/src/graph/` (15 files, ~4,354 lines Rust)
- GraphSearch tool: `codelet/tools/src/graph_search/` (4 files, ~210 lines)
- Handler: `codelet/napi/src/graph_search_handler.rs` (~103 lines)
- Schema: `codelet/napi/src/graph/schemas/agent-memory.pg`
- Residue methodology: See KGRAPH-019 attachments
- AST parsing: tree-sitter (existing in codebase), ast-grep (existing AstGrep tool)
