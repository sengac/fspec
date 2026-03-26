# GraphSearch Enhancement — CodeGraphContext Feature Parity Analysis

## Source Repository
- **Repo**: https://github.com/CodeGraphContext/CodeGraphContext
- **Cloned to**: `/tmp/CodeGraphContext` for analysis
- **Version analysed**: 0.3.1
- **License**: MIT

## Executive Summary

CodeGraphContext (CGC) is a Python-based MCP server + CLI that indexes code into a graph database (KùzuDB, FalkorDB, or Neo4j) for AI agents. Comparing it with fspec's nanograph-based GraphSearch reveals 10 significant feature gaps where CGC provides capabilities our system lacks.

## Architecture Comparison

| Aspect | fspec GraphSearch | CodeGraphContext |
|--------|-------------------|------------------|
| **Language** | Rust (nanograph, ast-grep) | Python (tree-sitter, Neo4j/KùzuDB) |
| **Storage** | Embedded nanograph (Lance/DataFusion/Arrow) | Neo4j / KùzuDB / FalkorDB |
| **AST Parser** | ast-grep (structural patterns) | tree-sitter (full AST walk) |
| **Query Language** | Custom nanograph `.gq` files | Cypher |
| **Graph Schema** | File, Module, Function, Type, Dependency | Repository, File, Module, Function, Class, Variable, Parameter |
| **Indexing** | Full overwrite on each run | Incremental with file watcher |
| **MCP** | Embedded in agent binary | Standalone MCP server |
| **Languages** | 14 | 18 (14 + Perl, Elixir, Haskell, Scala) |

## Feature Gap Matrix

| # | Feature | Priority | fspec Status | CGC Reference |
|---|---------|----------|-------------|---------------|
| 1 | Call chain path tracing | P0 | ❌ Missing | `code_finder.py:638-682` |
| 2 | Transitive callers/callees | P0 | ❌ Reserved but unimplemented | `code_finder.py:576-636` |
| 3 | Cyclomatic complexity | P0 | ❌ Missing | `code_finder.py:960-999` |
| 4 | Source code in nodes | P1 | ❌ Missing | `graph_builder.py:312-348` |
| 5 | Class hierarchy traversal | P1 | ❌ No dedicated action | `code_finder.py:449-510` |
| 6 | Incremental re-indexing | P1 | ❌ Full overwrite only | `watcher.py:1-203` |
| 7 | Variable/symbol tracking | P2 | ❌ Missing | `code_finder.py:86-98` |
| 8 | Full-text/content search | P2 | ❌ Client-side substring only | `code_finder.py:100-150` |
| 9 | Decorator/annotation search | P2 | ❌ Missing | `code_finder.py:257-281` |
| 10 | Portable graph bundles | P3 | ❌ Missing | `cgc_bundle.py` (entire file) |

## Child Work Units

| Card | Title | Priority |
|------|-------|----------|
| KGRAPH-060 | Call Chain / Path Tracing Between Two Functions | P0 |
| KGRAPH-061 | Transitive Callers / Callees (Multi-Hop Traversal) | P0 |
| KGRAPH-062 | Cyclomatic Complexity Analysis | P0 |
| KGRAPH-063 | Source Code and Metadata Storage in Graph Nodes | P1 |
| KGRAPH-064 | Class Hierarchy and Inheritance Traversal | P1 |
| KGRAPH-065 | Live File Watching and Incremental Re-indexing | P1 |
| KGRAPH-066 | Variable and Symbol Tracking | P2 |
| KGRAPH-067 | Full-Text and Content Search Within Graph (depends on KGRAPH-063) | P2 |
| KGRAPH-068 | Decorator and Annotation Search | P2 |
| KGRAPH-069 | Portable Graph Bundles — Export/Import | P3 |

## CGC Key Source Files Reference

| File | Lines | Purpose |
|------|-------|---------|
| `src/codegraphcontext/tools/code_finder.py` | 1009 lines | All query/analysis logic |
| `src/codegraphcontext/tools/graph_builder.py` | 1427 lines | Indexing, parsing, schema creation |
| `src/codegraphcontext/core/watcher.py` | 203 lines | File watching + incremental updates |
| `src/codegraphcontext/core/cgc_bundle.py` | ~31K chars | Bundle export/import |
| `src/codegraphcontext/tool_definitions.py` | 197 lines | MCP tool schemas |
| `src/codegraphcontext/server.py` | 299 lines | MCP server + request routing |
| `src/codegraphcontext/prompts.py` | 125 lines | LLM system prompt with SOPs |
| `src/codegraphcontext/tools/scip_indexer.py` | ~468 lines | SCIP protocol indexing (compiler-level accuracy) |

## Our Key Source Files Reference

| File | Purpose |
|------|---------|
| `codelet/tools/src/graph_search/types.rs` | GraphSearch action enum (9 variants) |
| `codelet/tools/src/graph_search/mod.rs` | Tool trait implementation |
| `codelet/tools/src/graph_search/handler.rs` | Per-session handler map |
| `codelet/napi/src/graph_search_handler.rs` | Action dispatch to AST/learnings |
| `codelet/napi/src/graph/database.rs` | GraphDatabase wrapper over nanograph |
| `codelet/napi/src/graph/registry.rs` | Named singleton graph registry |
| `codelet/napi/src/graph/dispatch_helpers.rs` | Shared query utilities |
| `codelet/napi/src/graph/graph_entities.rs` | JSONL entity serialization |
| `codelet/napi/src/ast_pipeline/` | AST extraction pipeline (14 languages) |

## Notably Good Ideas from CGC We Should Consider

### 1. LLM System Prompt with SOPs
CGC ships a `prompts.py` with Standard Operating Procedures that teach the LLM *how* to chain tools (SOP-1: locate → analyze → synthesize). We don't provide comparable guidance in the GraphSearch tool description.

### 2. SCIP Indexer for Compiler-Level Accuracy
CGC has an optional SCIP protocol integration (`scip_indexer.py`) that uses actual compilers (Pyright, tsc, rust-analyzer) for symbol resolution instead of heuristic AST pattern matching. This gives compiler-level accuracy for CALLS and INHERITS edges. Worth investigating for a future phase.

### 3. Repository-Scoped Queries
Every CGC query accepts an optional `repo_path` parameter to restrict results to a specific repository. Our system is single-project, but as we expand to indexing dependencies this becomes important.

### 4. Background Job System
CGC uses a `JobManager` with job IDs for long-running operations (indexing). The agent can `check_job_status` without blocking. Our `ast_index` is synchronous.
