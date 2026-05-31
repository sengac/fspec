# RPC-092 — Inventory of Files to Lift

**Card:** Lift `codelet/napi/src/graph/` → `codelet-graph`
**Generated:** 2026-05-28

All counts below are verbatim from
`find codelet/napi/src/graph -name '*.rs' -exec wc -l {} +` and
`ls codelet/napi/schemas/`.

---

## Summary

| Bucket                | Files | LOC     |
|-----------------------|-------|---------|
| `graph/` root         | 16    | 3,407   |
| `graph/ast_pipeline/` | 33    | 8,872   |
| `graph/ast_call_chain/` | 3   | 457     |
| Bundled schemas       | 4     | (data)  |
| **Total**             | **52 + 4 schemas** | **15,013 LOC** |

---

## `graph/` root (16 files, 3,407 LOC)

| File                                | LOC | Role                                                                                        |
|-------------------------------------|-----|---------------------------------------------------------------------------------------------|
| `mod.rs`                            | 195 | Module facade. `close_graph_db()`, `reset_graph_db()`, `extract_learnings_from_dag()`, `call_learnings_extraction_llm()`. **Touches `codelet_providers` — keep this dep edge.** |
| `registry.rs`                       | 183 | Named `GraphDatabase` singletons. `AST_CODE_GRAPH="ast-code"`, `LEARNINGS_GRAPH="learnings"`. Lazy init from `lazy_static`. Uses `include_str!("../../schemas/...")` — schemas path must follow the lift. |
| `database.rs`                       | 299 | `GraphDatabase` struct wrapping `nanograph::store::database::Database`. init/open/close/load_entities/run_query lifecycle. SHA-256 schema fingerprint logic. |
| `graph_entities.rs`                 | 134 | `GraphEntity` enum + `entities_to_jsonl()`. The wire format for the rest of the module.    |
| `bundle.rs`                         | 307 | Bundle export/import (ast_bundle / learnings_bundle) — `*.astbundle` files.                |
| `dispatch_helpers.rs`               | 146 | Shared format/match helpers used by both `ast_dispatch` and `learnings_dispatch`.          |
| `ast_dispatch.rs`                   | 392 | AST graph query router. `dispatch_ast_search`, `dispatch_ast_neighbors`, `dispatch_ast_stats`, etc. Loads `ast-queries.gq` via `include_str!`. |
| `ast_complexity.rs`                 | 222 | Cyclomatic complexity reporting.                                                            |
| `ast_dead_code.rs`                  | 281 | Dead-code report (unreferenced functions).                                                 |
| `ast_hierarchy.rs`                  | 222 | Type/class hierarchy traversal.                                                            |
| `ast_index.rs`                      | 394 | Full project re-index orchestration. Calls `ast_pipeline` extractors + `registry::get_graph`. |
| `ast_transitive.rs`                 | 140 | Transitive call/import closure helpers.                                                    |
| `learnings_dispatch.rs`             | 213 | Learnings graph query router. Loads `learnings-queries.gq`.                                |
| `learnings_context.rs`              | 238 | Per-session Learnings retrieval (decision lookup, related-learnings).                      |
| `learnings_extraction.rs`           | 301 | Residue-methodology LLM extraction pipeline. `LEARNINGS_EXTRACTION_PROMPT` const + JSON parsing. |
| `llm_response_parser.rs`            | 72  | Strips fences/preamble from LLM JSON output.                                                |
| `graph_reset_tests.rs`              | 359 | `#[cfg(test)]` integration tests (8) for `registry::reset_all_graphs`.                     |

---

## `graph/ast_pipeline/` (33 files, 8,872 LOC)

### Per-language AST extractors (14 files)

| File                       | LOC | Language     |
|----------------------------|-----|--------------|
| `ast_ts_extractor.rs`      | 941 | TypeScript / JavaScript / JSX / TSX |
| `ast_dart_extractor.rs`    | 953 | Dart         |
| `ast_go_extractor.rs`      | 540 | Go           |
| `ast_rust_extractor.rs`    | 501 | Rust         |
| `ast_java_extractor.rs`    | 497 | Java         |
| `ast_php_extractor.rs`     | 488 | PHP          |
| `ast_python_extractor.rs`  | 469 | Python       |
| `ast_scala_extractor.rs`   | 404 | Scala        |
| `ast_kotlin_extractor.rs`  | 400 | Kotlin       |
| `ast_csharp_extractor.rs`  | 328 | C#           |
| `ast_c_extractor.rs`       | 315 | C            |
| `ast_ruby_extractor.rs`    | 314 | Ruby         |
| `ast_cpp_extractor.rs`     | 262 | C++          |
| `ast_swift_extractor.rs`   | 218 | Swift        |

All depend on `ast-grep-core` + `ast-grep-language` (workspace).

### Per-ecosystem dependency extractors (12 files)

| File                              | LOC | Ecosystem                  |
|-----------------------------------|-----|----------------------------|
| `cargo_dep_extractor.rs`          | 155 | Rust (`Cargo.toml`)        |
| `java_dep_extractor.rs`           | 137 | Maven/Gradle (`pom.xml`, `build.gradle`) |
| `pubspec_dep_extractor.rs`        | 130 | Dart (`pubspec.yaml`)      |
| `pip_dep_extractor.rs`            | 125 | Python (`requirements.txt`, `pyproject.toml`) |
| `sbt_dep_extractor.rs`            | 88  | Scala (`build.sbt`)        |
| `gemfile_dep_extractor.rs`        | 81  | Ruby (`Gemfile`)           |
| `swift_dep_extractor.rs`          | 78  | Swift (`Package.swift`)    |
| `gomod_dep_extractor.rs`          | 75  | Go (`go.mod`)              |
| `csproj_dep_extractor.rs`         | 74  | C# (`*.csproj`)            |
| `npm_dep_extractor.rs`            | 58  | Node (`package.json`)      |
| `composer_dep_extractor.rs`       | 53  | PHP (`composer.json`)      |

### Pipeline infrastructure (7 files)

| File              | LOC | Role                                                                                          |
|-------------------|-----|-----------------------------------------------------------------------------------------------|
| `metadata.rs`     | 583 | File/function/type metadata extraction shared across language extractors.                     |
| `complexity.rs`   | 443 | Cyclomatic complexity scoring (shared by all extractors).                                     |
| `variables.rs`    | 435 | Variable extraction + scope tracking.                                                         |
| `mod.rs`          | 405 | Pipeline facade. `extract_codebase_async()`. Uses `ignore::WalkBuilder` for `.gitignore` walk. |
| `helpers.rs`      | 392 | Shared parsing helpers (e.g., `parse_decorator_text`, `cleanup_path`).                        |
| `edge_helpers.rs` | 380 | Edge construction helpers (Contains, Imports, DependsOn, etc.).                                |
| `incremental.rs`  | 232 | Incremental re-extraction (only files whose mtime changed since last index).                  |

---

## `graph/ast_call_chain/` (3 files, 457 LOC)

| File          | LOC | Role                                                                                  |
|---------------|-----|---------------------------------------------------------------------------------------|
| `mod.rs`      | 193 | Call-chain facade. `dispatch_ast_call_chain`, `dispatch_ast_callers`, `dispatch_ast_callees`. |
| `bfs.rs`      | 180 | Breadth-first traversal over `Calls` edges with cycle detection.                       |
| `snapshot.rs` | 84  | Captures current graph state for stable call-chain queries.                            |

---

## Bundled schemas (`codelet/napi/schemas/`)

These are loaded at compile time via `include_str!("../../schemas/...")`
from inside the graph module. They must move with the lift.

| File                    | Purpose                                  |
|-------------------------|------------------------------------------|
| `ast-code.pg`           | Property-graph schema for AST graph.     |
| `learnings.pg`          | Property-graph schema for Learnings graph. |
| `ast-queries.gq`        | Named `nanograph` queries for AST graph. |
| `learnings-queries.gq`  | Named `nanograph` queries for Learnings graph. |

**Lift target:** `codelet/graph/schemas/`. The four `include_str!`
call-sites in `registry.rs`, `ast_dispatch.rs`, and
`learnings_dispatch.rs` need their relative paths updated.

---

## Dependents inside `codelet/napi/src/`

Files outside `graph/` that touch `crate::graph::*` (from prior
`grep -l 'graph_search_handler\|graph::'` audit):

| File                       | Touches                                                                      |
|----------------------------|------------------------------------------------------------------------------|
| `agent_loop.rs`            | `register_deep_search_handler(...)`, `graph_search_handler::create_handler()` — call sites inside the body the RPC-072 Phase B port needs. |
| `deep_search_handler.rs`   | Uses `graph_search_handler::create_handler` via `register_*`.                |
| `graph_search_handler.rs`  | `crate::graph::{database::GraphDatabase, ast_dispatch, learnings_dispatch, registry}`. |
| `lib.rs`                   | `pub mod graph;` re-export.                                                  |
| `session_bindings.rs`      | `crate::graph::registry` (for `set_data_directory` reset).                   |
| `test_support.rs`          | `crate::graph::registry` (test fixture reset).                               |

After the lift, all of these become `codelet_graph::*` imports. The
`codelet-napi` crate keeps a thin `pub use codelet_graph::*;`
re-export shim in `codelet/napi/src/graph.rs` (replacing the directory)
so the 24 existing `codelet/napi/tests/ast_*_test.rs` integration tests
keep compiling.

---

## Integration test surface to preserve

24 NAPI integration tests under `codelet/napi/tests/` depend on the
graph module:

```
ast_call_chain_test.rs               ast_incremental_test.rs
ast_class_import_crash_test.rs        ast_index_custom_path_test.rs
ast_complexity_test.rs                ast_metadata_test.rs
ast_dead_code_test.rs                 ast_multi_language_extraction_test.rs
ast_decorator_search_test.rs          ast_query_interface_test.rs
ast_dependency_population_test.rs     ast_search_filter_test.rs
ast_edge_quality_test.rs              ast_transitive_test.rs
ast_export_import_test.rs             ast_variable_tracking_test.rs
ast_extraction_pipeline_test.rs       deprecate_old_graph_test.rs
ast_graph_data_model_test.rs         + 6 more
ast_hierarchy_test.rs
```

**Acceptance bar:** all 24 tests pass post-lift with `cargo test -p codelet-napi`.

Additionally, the existing in-module `graph_reset_tests.rs` (8 tests)
moves with the source.
