# RPC-092 — Architecture

**Card:** Lift `codelet/napi/src/graph/` → `codelet-graph`
**Generated:** 2026-05-28

---

## Workspace topology — before and after

### Before (today)

```
codelet (workspace root)
├── agent-loop/        (NAPI-free; Phase A complete)
├── cli/
├── common/
├── core/
├── fspec/             (the binary; NAPI-free)
├── fspec-tui/
├── git/
├── napi/              ← graph/ lives in here ◀━━━━━━━┓
│   └── src/                                            ┃
│       └── graph/     (52 files, 15,013 LOC)           ┃ blocks
├── providers/                                          ┃ RPC-072
├── rpc/                                                ┃ Phase B
├── sessions/                                           ┃
├── tools/                                              ┃
└── tui/                                                ┃
                                                       ━┛
```

### After (post-RPC-092)

```
codelet (workspace root)
├── agent-loop/        (NAPI-free; can now import codelet-graph)
├── cli/
├── common/
├── core/
├── fspec/
├── fspec-tui/
├── git/
├── graph/             ◀━━━━ NEW: peer workspace crate, NAPI-free
│   ├── Cargo.toml
│   ├── schemas/
│   │   ├── ast-code.pg
│   │   ├── ast-queries.gq
│   │   ├── learnings.pg
│   │   └── learnings-queries.gq
│   ├── src/
│   │   ├── lib.rs                  (was graph/mod.rs)
│   │   ├── registry.rs
│   │   ├── database.rs
│   │   ├── graph_entities.rs
│   │   ├── bundle.rs
│   │   ├── dispatch_helpers.rs
│   │   ├── ast_dispatch.rs
│   │   ├── ast_complexity.rs
│   │   ├── ast_dead_code.rs
│   │   ├── ast_hierarchy.rs
│   │   ├── ast_index.rs
│   │   ├── ast_transitive.rs
│   │   ├── ast_call_chain/
│   │   │   ├── mod.rs
│   │   │   ├── bfs.rs
│   │   │   └── snapshot.rs
│   │   ├── ast_pipeline/           (33 files)
│   │   ├── learnings_dispatch.rs
│   │   ├── learnings_context.rs
│   │   ├── learnings_extraction.rs
│   │   └── llm_response_parser.rs
│   └── tests/
│       ├── no_napi_dependency.rs   (RPC-067 boundary guard)
│       └── graph_reset_test.rs     (was graph_reset_tests.rs)
├── napi/
│   └── src/
│       └── graph.rs                ◀━━━━ REPLACES graph/ directory;
│                                            thin `pub use codelet_graph::*;` shim
├── providers/
├── rpc/
├── sessions/
├── tools/
└── tui/
```

---

## Dependency rules (post-lift)

### Crates that may depend on `codelet-graph`

- `codelet-napi` — keeps the shim re-export so existing NAPI bindings
  + 24 ast_*_test.rs integration tests keep working.
- `codelet-agent-loop` — adds the dep so it can host
  `deep_search_handler` + `graph_search_handler`.
- `codelet-tools` — already a dep of both consumers; no change needed.

### Crates `codelet-graph` may depend on

- `nanograph` (workspace) — the underlying graph DB.
- `codelet-providers` (workspace) — only for
  `call_learnings_extraction_llm`. This is the single non-trivial
  external dep already present in the existing module.
- `ast-grep-core` + `ast-grep-language` (workspace) — per-language extractors.
- `ignore` (workspace) — gitignore-aware file walks.
- `chrono`, `globset`, `serde`, `serde_json`, `sha2`, `tracing`,
  `lazy_static`, `uuid` (all workspace).
- `arrow-array`, `arrow-schema` (KGRAPH-069 export).

### Forbidden arrows

- `codelet-graph` → `codelet-napi` (FORBIDDEN) — the whole point.
- `codelet-graph` → `napi`/`napi-derive` (FORBIDDEN).
- `codelet-graph` → `codelet-fspec`, `codelet-fspec-tui`, `codelet-agent-loop`,
  `codelet-sessions`, `codelet-tui` (FORBIDDEN — these are all upstream
  consumers; only one direction).

### Boundary test

New file `codelet/graph/tests/no_napi_dependency.rs` (mirrors
`codelet/agent-loop/tests/no_napi_dependency.rs` per RPC-067) shells out
to `cargo metadata --no-deps --format-version 1` and asserts that the
`codelet-graph` package's `dependencies` array contains zero edges to
the forbidden set above.

---

## Public API surface

### `lib.rs` (was `mod.rs`)

```rust
//! codelet-graph — embedded graph databases for the AST and Learnings graphs.
//!
//! Hosts the dual-graph architecture:
//!   - "ast-code": project-scoped code graph at <project>/.fspec/graph/ast-code.nano/
//!   - "learnings": global learnings graph at ~/.fspec/graph/learnings.nano/

pub mod ast_call_chain;
pub mod ast_complexity;
pub mod ast_dead_code;
pub mod ast_dispatch;
pub mod ast_hierarchy;
pub mod ast_index;
pub mod ast_pipeline;
pub mod ast_transitive;
pub mod bundle;
pub mod database;
pub mod dispatch_helpers;
pub mod graph_entities;
pub mod learnings_context;
pub mod learnings_dispatch;
pub mod learnings_extraction;
pub mod llm_response_parser;
pub mod registry;

pub fn close_graph_db() { registry::close_all_graphs(); }
pub fn reset_graph_db() { registry::reset_all_graphs(); }
pub async fn extract_learnings_from_dag(dag_text: &str, llm_response: Option<&str>) { /* … */ }
pub async fn call_learnings_extraction_llm(
    provider_name: &str,
    model_id: Option<&str>,
    dag_text: &str,
) -> Option<String> { /* … */ }
```

### Path rewrites (mechanical)

Every `super::*`, `crate::graph::*` inside the lifted files needs the
following transform — and **only** this transform:

| Before                                | After                                  |
|---------------------------------------|----------------------------------------|
| `use crate::graph::database::…`       | `use crate::database::…`               |
| `use crate::graph::registry::…`       | `use crate::registry::…`               |
| `use crate::graph::ast_dispatch::…`   | `use crate::ast_dispatch::…`           |
| `use super::graph_entities::…`        | `use super::graph_entities::…`         |
| `include_str!("../../schemas/ast-code.pg")` | `include_str!("../schemas/ast-code.pg")` |
| `include_str!("../../schemas/learnings.pg")` | `include_str!("../schemas/learnings.pg")` |
| `include_str!("../../schemas/ast-queries.gq")` | `include_str!("../schemas/ast-queries.gq")` |
| `include_str!("../../schemas/learnings-queries.gq")` | `include_str!("../schemas/learnings-queries.gq")` |

(The `include_str!` paths change because the schemas move from
`codelet/napi/schemas/` to `codelet/graph/schemas/`, and the call-site
files move from `codelet/napi/src/graph/*.rs` to `codelet/graph/src/*.rs`.)

### NAPI shim (`codelet/napi/src/graph.rs`)

Replaces the deleted `codelet/napi/src/graph/` directory:

```rust
//! Thin re-export of codelet-graph for backward compatibility with
//! existing NAPI bindings and the 24 ast_*_test.rs integration tests.
//!
//! New code in the workspace should depend on `codelet-graph` directly.
//! This shim exists solely so the codelet-napi crate keeps compiling
//! during the RPC-092 transition window.

pub use codelet_graph::*;
```

Plus a `codelet-graph = { path = "../graph" }` line in
`codelet/napi/Cargo.toml`.

---

## Schema-file location decision

The four schema files (`*.pg`, `*.gq`) move with the source code. They
are referenced by `include_str!`, so they are compiled into the binary
at build time — there is no runtime dependency on their location on
disk. The new home `codelet/graph/schemas/` keeps them co-located with
the only crate that reads them.

---

## Why `learnings_extraction.rs` keeps its `codelet-providers` dep

`mod.rs::call_learnings_extraction_llm` (90 LOC) creates a minimal
`rig` agent against the same `ProviderManager` the rest of the
workspace uses. Pulling this responsibility out of the graph crate
would require a callback trait + caller wiring at every site that
extracts learnings. Cost: weeks. Benefit: marginal — the dep already
exists in the workspace dep graph. Decision: keep it.

The acceptance bar is "no `codelet-napi` edge", not "no provider edge".

---

## Consumer wiring after the lift

```text
                ┌─────────────────────┐
                │   codelet-graph     │
                │   (NEW peer crate)  │
                └─────────────────────┘
                       ▲       ▲
                       │       │
        ┌──────────────┘       └────────────────────┐
        │                                            │
┌───────────────────┐                  ┌──────────────────────┐
│   codelet-napi    │                  │  codelet-agent-loop  │
│  (graph.rs shim,  │                  │  (RPC-072 Phase B    │
│   24 tests, etc.) │                  │   can now use graph) │
└───────────────────┘                  └──────────────────────┘
        │                                            │
        ▼                                            ▼
   (Node binding)                       ┌──────────────────────┐
                                        │     codelet-fspec    │
                                        │      (the binary)    │
                                        └──────────────────────┘
```

---

## What this card does NOT do

- Does NOT lift `deep_search_handler.rs` (that's RPC-072 Phase B work,
  unblocked by this card).
- Does NOT lift `graph_search_handler.rs` (same).
- Does NOT change any graph behaviour — it is a pure relocation.
- Does NOT delete the NAPI bindings — those keep working through the
  shim until a future card formally removes `codelet-napi`.

The whole card is a mechanical relocation + boundary test + path rewrite.
The verbatim-lift discipline from Phase A applies in full.
