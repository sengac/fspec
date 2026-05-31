# RPC-092 — Root Cause Analysis

**Card:** Lift `codelet/napi/src/graph/` into NAPI-free `codelet-graph` crate
**Discovered:** 2026-05-28 (during RPC-072 Phase A → Phase B transition)
**Blocks:** RPC-072 (Work Agent round-trip) Phase B
**Author:** ACDD session, 2026-05-28

---

## TL;DR

RPC-072 Phase A successfully lifted ~5,400 LOC of NAPI agent-loop machinery
into the NAPI-free `codelet-agent-loop` crate. Phase B (porting the agent
loop **body** — `agent_loop.rs:299-1456`) cannot proceed because the body
calls two handler factories whose transitive closure pulls **the entire
graph module** (52 source files, ~15,013 LOC) into the binary. The graph
module is not technically NAPI-coupled, but it lives under
`codelet/napi/src/graph/` and is only accessible via `crate::graph::*`
from inside the `codelet-napi` crate.

This card lifts the graph module **as a peer workspace crate**
(`codelet-graph`) so:

1. `codelet-agent-loop` can depend on it directly.
2. The NAPI-free dependency boundary (RPC-067 `no_napi_dependency`
   guard test) stays intact.
3. The 24 existing `codelet/napi/tests/ast_*_test.rs` files keep working
   through a thin NAPI re-export shim during the transition.

---

## How we got here

### Discovery sequence (this session)

1. **RPC-072 reopened** (turn 0): The previous "done" version was a
   203-line minimum-viable stub. 13 functional gaps catalogued. Full
   1,769-LOC NAPI source `codelet/napi/src/agent_loop.rs` is the
   canonical target.
2. **Phase A lift completed** (turns 1-634):
   `persist.rs (298) + thinking_config.rs (287) + thinking_level_detection.rs (383)
   + dispatch.rs (215, run_with_provider! macro) + inject_summary_handler.rs (281)
   + background_output.rs (362) + schedule_handler.rs (374)
   + session_search_handler.rs (1606) + agent_manager_handler.rs (1282)
   + stream_chunk_json.rs (216)` — all built green, 38 unit tests pass,
   checkpoint `phase-a-lift-complete` committed.
3. **Phase B blocker surfaced** (turns 635-651): When `lib.rs`
   attempted to add `pub mod deep_search_handler;` (the Phase B
   prerequisite), `cargo check` failed with `E0432` unresolved imports
   on:
   ```
   crate::graph::registry
   crate::graph::ast_dispatch
   crate::graph::learnings_dispatch
   ```
   Inspection of `graph_search_handler.rs` revealed it touches
   `crate::graph::database::GraphDatabase` directly.

### Why the obvious "just add it" doesn't work

`codelet-agent-loop` has a hard architectural invariant: **zero
dependency edges to `codelet-napi`** (enforced by
`codelet/agent-loop/tests/no_napi_dependency.rs`, dependency rule from
RPC-067). Adding `codelet-napi` to `[dependencies]` to access the graph
module would:

- Break the RPC-067 forbidden-arrow boundary.
- Pull `napi`, `napi-derive`, NAPI runtime symbols into the Rust
  binary (`fspec` workspace member), bloating the binary by megabytes
  and re-introducing the Node-runtime coupling the whole RPC-030/RPC-072
  effort is designed to eliminate.
- Defeat the purpose of the lift.

### Why stub handlers ("Option B" — rejected)

Earlier in the session I floated a `BridgeHandlers` trait with a
`NoopBridgeHandlers` impl that would let the agent loop body compile
without graph access. The user explicitly rejected this pattern:
RPC-072 Rule [5] mandates **zero functional cut-downs**. A
`NoopBridgeHandlers` would silently swallow `GraphSearch` and
`DeepSearch` tool calls — exactly the kind of gap that put the
original 203-line stub in the broken state we are now repairing.

### Why "lift the graph module here-and-now inside Phase B" doesn't work

Attempted at turn 635. Rolled back at turn 641. Reasons:

1. The graph module is **52 files, ~15,013 LOC** — comparable in scope
   to Phase A itself.
2. Mixing the graph lift into the Phase B agent-loop body port would
   produce a single ~30,000-LOC commit with no checkpoint between
   stages — context-window-exhausting and impossible to bisect.
3. The graph module has its own 24-test integration suite that needs
   to keep passing throughout the transition. That deserves its own
   parity test plan and its own card.

---

## The right thing

Carve **RPC-092** as a discrete card with a discrete parity bar:

| Aspect            | Status after RPC-092                                            |
|-------------------|------------------------------------------------------------------|
| Crate created     | `codelet/graph/` (workspace member `codelet-graph`)              |
| Files lifted      | All 52 `.rs` files verbatim from `codelet/napi/src/graph/`       |
| Schemas lifted    | `ast-code.pg`, `learnings.pg`, `ast-queries.gq`, `learnings-queries.gq` move to `codelet/graph/schemas/` |
| NAPI boundary     | New `codelet/graph/tests/no_napi_dependency.rs` mirrors the RPC-067 guard |
| Tests             | All 24 `codelet/napi/tests/ast_*_test.rs` still green via thin shim |
| Downstream wiring | `codelet-agent-loop` adds `codelet-graph` as a dep              |
| **RPC-072 Phase B** | **Unblocked** — `deep_search_handler.rs` + `graph_search_handler.rs` can now be lifted into `codelet-agent-loop` |

---

## Why the graph module is liftable

A `grep` audit of the entire graph subtree confirms it has **zero
direct NAPI coupling**:

```bash
$ cd codelet/napi/src/graph && \
    grep -l "napi\|node_bindgen\|N-API" *.rs ast_pipeline/*.rs ast_call_chain/*.rs
# (no matches)
```

External crate imports are limited to:

- `nanograph` (graph DB) — already workspace-versioned.
- `chrono`, `globset`, `serde`, `serde_json`, `sha2`, `tracing`,
  `lazy_static` — all already at workspace scope.
- `ignore` — already at workspace scope.
- `codelet-providers` (only inside `mod.rs::call_learnings_extraction_llm`)
  — already in the workspace dep graph.

In other words, the only thing keeping the graph module captive inside
`codelet-napi` is its **module path** (`crate::graph::*`). The lift is
mechanical: change `crate::graph::*` → `codelet_graph::*`, move
files, register new workspace member, re-run tests.

---

## Why this matters for the bigger picture

The 2026-05-28 screenshot the user pasted at turn 325 — assistant
replies missing, "API Error: 429..." raw JSON in the scrollback — is the
end-state symptom. RPC-078 (scrollback wrap) and RPC-079 (dialog
wrappers) were painting over that symptom rather than fixing it. The
fix is RPC-072's agent loop body port. The body port is blocked on
graph-handler access. Graph-handler access is blocked on this card.

Carving RPC-092 keeps each lift coherent, each checkpoint atomic, and
honours the "zero cut-downs" invariant the user has been consistent on.
