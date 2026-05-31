# RPC-092 — Test Plan

**Card:** Lift `codelet/napi/src/graph/` → `codelet-graph`
**Generated:** 2026-05-28

This card is a **mechanical relocation** — no behaviour changes. The
test plan therefore reduces to: (a) **preserve every test that exists
today**, (b) **add the boundary test** that enforces the lift's whole
reason for being, (c) **prove the downstream shim works**.

---

## Test pyramid

```
            ┌─────────────────────────────────────┐
            │  Tier 4: workspace clippy + check   │   ← gate everything
            │  (cargo clippy --workspace          │
            │   -- -D warnings)                   │
            └─────────────────────────────────────┘
            ┌─────────────────────────────────────┐
            │  Tier 3: downstream NAPI test suite │   ← 24 tests, must still pass
            │  (codelet/napi/tests/ast_*_test.rs) │      via shim re-export
            └─────────────────────────────────────┘
            ┌─────────────────────────────────────┐
            │  Tier 2: lifted graph_reset_test    │   ← 8 tests, must pass
            │  (codelet/graph/tests/              │      now as integration tests
            │   graph_reset_test.rs)              │
            └─────────────────────────────────────┘
            ┌─────────────────────────────────────┐
            │  Tier 1: NEW boundary guard         │   ← THE acceptance bar
            │  (codelet/graph/tests/              │
            │   no_napi_dependency.rs)            │
            └─────────────────────────────────────┘
```

---

## Tier 1 — Boundary guard (NEW)

**File:** `codelet/graph/tests/no_napi_dependency.rs`
**Pattern:** mirrors `codelet/agent-loop/tests/no_napi_dependency.rs`
(written for RPC-067 / Phase A).

**What it asserts:**

1. `cargo metadata --no-deps` for `codelet-graph` contains zero
   dependencies on `codelet-napi`.
2. Same for direct `napi` crate.
3. Same for `napi-derive`.

**Why this is the acceptance bar:**
The whole point of the lift is to make `codelet-agent-loop` able to
depend on `codelet-graph` without re-introducing the NAPI runtime. If
this test fails, the lift is broken — regardless of how green everything
else is.

**Acceptance criteria:**
```
cargo test -p codelet-graph --test no_napi_dependency
# expected: test codelet_graph_has_no_napi_dependency ... ok
```

---

## Tier 2 — Lifted reset tests (PRESERVED)

**Source today:** `codelet/napi/src/graph/graph_reset_tests.rs`
(359 LOC, 8 `#[cfg(test)]` tests inside a `mod tests {}` block).

**Target post-lift:** `codelet/graph/tests/graph_reset_test.rs`
(standalone integration test using `codelet_graph::*` imports).

**What it covers (preserved verbatim):**

| Test name                                              | What it proves                                                  |
|--------------------------------------------------------|------------------------------------------------------------------|
| `test_reset_clears_in_memory_registry`                 | `reset_all_graphs()` empties the `lazy_static` registry.        |
| `test_reset_clears_on_disk_database`                   | The `*.nano/` directory is fully removed after reset.           |
| `test_reset_handles_missing_database_directory`        | No panic when the dir doesn't exist yet.                        |
| `test_reset_allows_schema_change`                      | After reset, init with a new schema succeeds.                   |
| `test_reset_does_not_affect_other_graphs`              | Resetting `"ast-code"` does not touch `"learnings"`.            |
| `test_reset_with_concurrent_access_is_safe`            | Mutex contention does not corrupt the registry.                 |
| `test_init_after_reset_creates_fresh_database`         | Post-reset `get_graph(name)` returns a fresh handle.            |
| `test_close_then_reset_is_idempotent`                  | `close_graph_db()` followed by `reset_graph_db()` does not panic. |

**Acceptance criteria:**
```
cargo test -p codelet-graph --test graph_reset_test
# expected: 8 passed, 0 failed
```

---

## Tier 3 — Downstream NAPI integration tests (PRESERVED via shim)

These 24 tests live in `codelet/napi/tests/` and depend on the graph
module via `crate::graph::*`. After the lift, they depend on the same
items via `codelet_napi::graph::*`, which is the thin re-export shim:

```rust
// codelet/napi/src/graph.rs (NEW file replacing the directory):
pub use codelet_graph::*;
```

**Tests that must still pass (24 total):**

```
ast_call_chain_test.rs                  ast_hierarchy_test.rs
ast_class_import_crash_test.rs          ast_incremental_test.rs
ast_complexity_test.rs                  ast_index_custom_path_test.rs
ast_dead_code_test.rs                   ast_metadata_test.rs
ast_decorator_search_test.rs            ast_multi_language_extraction_test.rs
ast_dependency_population_test.rs       ast_query_interface_test.rs
ast_edge_quality_test.rs                ast_search_filter_test.rs
ast_export_import_test.rs               ast_transitive_test.rs
ast_extraction_pipeline_test.rs         ast_variable_tracking_test.rs
ast_graph_data_model_test.rs            deprecate_old_graph_test.rs
                                       + 4 KGRAPH/Learnings tests
```

**Acceptance criteria:**
```
cargo test -p codelet-napi --tests
# expected: all integration tests pass, zero regressions vs pre-lift baseline
```

**Smoke check during dev** (single test as fast feedback loop):
```
cargo test -p codelet-napi --test ast_extraction_pipeline_test
```

---

## Tier 4 — Workspace gate

After every phase of the implementation plan, the lift is not "done"
until the whole workspace builds clean and clippy-clean:

```
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace --no-fail-fast
```

The clippy bar matters because `codelet/Cargo.toml` declares the lint
rules at workspace scope (no `unwrap_used`, no `expect_used`, no
`panic`, etc.). The lifted code already passes them, but the path
rewrites are an opportunity to introduce a regression.

---

## Negative tests (anti-regression)

These are NOT new tests — they are existing checks that must continue
to pass post-lift. Listing them here to make sure they're not skipped
during validation.

1. **RPC-067 forbidden arrows on existing crates** — e.g.
   `codelet/fspec/tests/no_napi_dependency.rs`,
   `codelet/agent-loop/tests/no_napi_dependency.rs`. Adding
   `codelet-graph` to the dep graph must not accidentally add a NAPI
   edge anywhere.

2. **The 38 unit tests in `codelet-agent-loop`** (Phase A lift) —
   `cargo test -p codelet-agent-loop --lib`. These exercise the
   modules lifted in Phase A and don't touch graph code, so they must
   stay green.

3. **The full `codelet-fspec` test suite** —
   `cargo test -p codelet-fspec`. The binary doesn't yet pull graph
   code directly, but its dep closure (via `codelet-sessions`,
   `codelet-tui`, etc.) gets a new transitive edge. Must stay green.

---

## Manual smoke test (post-merge)

After the lift, run the existing fspec CLI with a graph operation:

```bash
./fspec graph-index .
./fspec graph-search-stats
./fspec graph-search "Authentication"
```

Each command must produce the same output as it did pre-lift. If any
diverge, the lift has changed behaviour and must be re-audited for an
accidental edit during path rewrites.

---

## Coverage linking (per fspec workflow)

Each scenario in `spec/features/codelet-graph-crate-lift.feature` must
be linked with `fspec link-coverage` to the matching test:

| Scenario | Test file |
|----------|-----------|
| `codelet-graph crate exists as a workspace member` | `codelet/graph/Cargo.toml` (implFile) + `codelet/graph/tests/no_napi_dependency.rs` (testFile) |
| `codelet-graph has zero dependency on codelet-napi` | `codelet/graph/tests/no_napi_dependency.rs` |
| `Graph reset preserves on-disk semantics after lift` | `codelet/graph/tests/graph_reset_test.rs` |
| `Existing NAPI ast_extraction_pipeline_test still passes` | `codelet/napi/tests/ast_extraction_pipeline_test.rs` |
| `Existing NAPI ast_query_interface_test still passes` | `codelet/napi/tests/ast_query_interface_test.rs` |
| `codelet-agent-loop can import codelet-graph cleanly` | (smoke test added in `codelet/agent-loop/tests/codelet_graph_import_smoke.rs`) |
| `RPC-072 Phase B is unblocked` | `spec/attachments/RPC-072/agent-loop-parity-gap.md` (manual cross-reference) |

---

## "Done" definition

RPC-092 is `done` when **all** of the following are true:

- [ ] `codelet/graph/` exists as a workspace member.
- [ ] All 52 source files lifted verbatim (verified by `git log --follow` blame preservation).
- [ ] All 4 schema files lifted to `codelet/graph/schemas/`.
- [ ] `cargo test -p codelet-graph --test no_napi_dependency` passes.
- [ ] `cargo test -p codelet-graph --test graph_reset_test` passes (8 tests).
- [ ] `cargo test -p codelet-napi --tests` passes (all 24 ast_*_test.rs files).
- [ ] `cargo clippy --workspace -- -D warnings` is clean.
- [ ] The shim file `codelet/napi/src/graph.rs` exists and re-exports `codelet-graph`.
- [ ] `codelet/napi/Cargo.toml` adds `codelet-graph = { path = "../graph" }`.
- [ ] The manual smoke test (`./fspec graph-index .` + family) produces
      identical output to the pre-lift baseline.
- [ ] `RPC-072` board state shows it ready to move from `blocked` →
      `testing` (i.e. the `blockedBy: RPC-092` edge is satisfied).
- [ ] Coverage for `spec/features/codelet-graph-crate-lift.feature` is 100%.
