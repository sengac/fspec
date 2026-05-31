# RPC-092 — Implementation Plan

**Card:** Lift `codelet/napi/src/graph/` → `codelet-graph`
**Generated:** 2026-05-28
**Estimated effort:** 8 story points (Phase 1-4 mechanical + Phase 5 verification)

---

## Phase 0 — Pre-flight (DO THIS FIRST)

Before any file moves. Each step is a checkpointable boundary.

1. **Create a baseline checkpoint** so any step is restorable:
   ```
   fspec checkpoint RPC-092 rpc092-pre-lift-baseline
   ```
2. **Verify the current state is green:**
   ```bash
   cargo check -p codelet-napi
   cargo check -p codelet-agent-loop
   cargo test -p codelet-napi --tests ast_extraction_pipeline_test -- --nocapture  # smoke test
   ```
3. **Audit the lift target one more time:**
   ```bash
   find codelet/napi/src/graph -name '*.rs' | wc -l          # expect 52
   grep -l 'napi\|node_bindgen\|N-API' \
       codelet/napi/src/graph/*.rs \
       codelet/napi/src/graph/ast_pipeline/*.rs \
       codelet/napi/src/graph/ast_call_chain/*.rs            # expect (no output)
   ```
4. **Snapshot the cargo dep graph** for diff comparison later:
   ```bash
   cargo metadata --no-deps --format-version 1 > /tmp/rpc092-before.json
   ```

---

## Phase 1 — Scaffold the `codelet-graph` crate (no file moves yet)

Goal: make `codelet-graph` exist as an empty workspace crate that
builds clean. No graph code in it yet.

1. **Create directory + skeleton:**
   ```bash
   mkdir -p codelet/graph/src codelet/graph/schemas codelet/graph/tests
   ```
2. **Write `codelet/graph/Cargo.toml`:**

   ```toml
   [package]
   name = "codelet-graph"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   description = "Embedded graph databases for the AST and Learnings dual-graph architecture."

   [dependencies]
   # Graph DB
   nanograph = "1.0.0"

   # Provider — only used by call_learnings_extraction_llm
   codelet-providers = { workspace = true }

   # AST extractors
   ast-grep-core = { workspace = true }
   ast-grep-language = { workspace = true }

   # File walking
   ignore = { workspace = true }

   # Utilities
   chrono = { workspace = true }
   globset = { workspace = true }
   serde = { workspace = true }
   serde_json = { workspace = true }
   sha2 = { workspace = true }
   tracing = { workspace = true }
   lazy_static = { workspace = true }
   uuid = { workspace = true }

   # Arrow types for export
   arrow-array = "57"
   arrow-schema = "57"

   # rig — used inside learnings extraction LLM call
   rig-core = { version = "0.28.0", default-features = false }

   [dev-dependencies]
   tempfile = { workspace = true }
   tokio = { workspace = true }
   codelet-test-helpers = { workspace = true }

   [lints]
   workspace = true
   ```

3. **Write a placeholder `codelet/graph/src/lib.rs`:**
   ```rust
   //! codelet-graph — embedded graph databases (RPC-092 scaffold).
   //!
   //! Modules will be filled in by Phase 2 of the lift.
   ```

4. **Register the workspace member.** Edit `codelet/Cargo.toml`:
   - Add `"graph",` to `[workspace] members`.
   - Add `codelet-graph = { path = "graph" }` to `[workspace.dependencies]`.

5. **Smoke build:**
   ```bash
   cargo check -p codelet-graph
   ```
   Must succeed against the empty crate.

6. **Checkpoint:**
   ```
   fspec checkpoint RPC-092 rpc092-scaffold
   ```

---

## Phase 2 — Move schemas

Schemas are small (4 files), have no Rust dependencies of their own, and
are the cleanest first step.

1. **Move the files:**
   ```bash
   git mv codelet/napi/schemas/ast-code.pg          codelet/graph/schemas/
   git mv codelet/napi/schemas/ast-queries.gq       codelet/graph/schemas/
   git mv codelet/napi/schemas/learnings.pg         codelet/graph/schemas/
   git mv codelet/napi/schemas/learnings-queries.gq codelet/graph/schemas/
   ```
2. **Update the four `include_str!` call sites** inside
   `codelet/napi/src/graph/registry.rs`, `ast_dispatch.rs`, and
   `learnings_dispatch.rs` (one each — see
   [`architecture.md` § Path rewrites](architecture.md)).

   These updates are temporary — they'll move with the source in
   Phase 3 — but they need to be correct for the transient build
   between Phase 2 and Phase 3.

3. **Verify:**
   ```bash
   cargo check -p codelet-napi
   ```
4. **Checkpoint:**
   ```
   fspec checkpoint RPC-092 rpc092-schemas-moved
   ```

---

## Phase 3 — Move sources (verbatim)

This is the big mechanical step. Done in a single commit because cargo
needs all the source to be in one consistent place for the build to
succeed.

1. **Move files with `git mv` (preserves blame):**

   ```bash
   # ast_call_chain/
   git mv codelet/napi/src/graph/ast_call_chain codelet/graph/src/ast_call_chain

   # ast_pipeline/
   git mv codelet/napi/src/graph/ast_pipeline   codelet/graph/src/ast_pipeline

   # Root-level files (one-by-one for clarity)
   for f in ast_complexity.rs ast_dead_code.rs ast_dispatch.rs ast_hierarchy.rs \
            ast_index.rs ast_transitive.rs bundle.rs database.rs dispatch_helpers.rs \
            graph_entities.rs learnings_context.rs learnings_dispatch.rs \
            learnings_extraction.rs llm_response_parser.rs registry.rs; do
     git mv "codelet/napi/src/graph/$f" "codelet/graph/src/$f"
   done

   # mod.rs becomes lib.rs
   git mv codelet/napi/src/graph/mod.rs codelet/graph/src/lib.rs

   # tests
   git mv codelet/napi/src/graph/graph_reset_tests.rs \
          codelet/graph/tests/graph_reset_test.rs
   ```

2. **Apply the mechanical path rewrites** (from
   [`architecture.md` § Path rewrites](architecture.md)):
   - `crate::graph::*` → `crate::*` (the lifted crate's root is now
     the old `crate::graph::` namespace).
   - `super::*` references stay valid because the directory layout is
     preserved.
   - `include_str!("../../schemas/...")` → `include_str!("../schemas/...")`
     (one fewer `..` because we moved from `src/graph/*.rs` to `src/*.rs`).

3. **Convert `graph_reset_tests.rs` from `#[cfg(test)] mod tests`
   into a standalone integration test** under
   `codelet/graph/tests/graph_reset_test.rs`:
   - Strip the `#[cfg(test)] mod tests { ... }` wrapper.
   - Change `use crate::graph::database::GraphDatabase` →
     `use codelet_graph::database::GraphDatabase`.
   - Change `use crate::graph::registry` → `use codelet_graph::registry`.

4. **Write the NAPI shim** at `codelet/napi/src/graph.rs`
   (replaces the now-deleted directory):

   ```rust
   //! Thin re-export of codelet-graph for the existing NAPI bindings
   //! and the 24 codelet/napi/tests/ast_*_test.rs integration tests.
   //! New code in the workspace should depend on codelet-graph directly.

   pub use codelet_graph::*;
   ```

5. **Add the dep to `codelet/napi/Cargo.toml`:**
   ```toml
   codelet-graph = { path = "../graph" }
   ```

6. **Verify the build:**
   ```bash
   cargo check -p codelet-graph
   cargo check -p codelet-napi
   cargo check -p codelet-agent-loop          # still passes
   cargo check --workspace                    # all crates build
   ```

7. **Checkpoint:**
   ```
   fspec checkpoint RPC-092 rpc092-sources-moved
   ```

---

## Phase 4 — Boundary test + tests pass

1. **Write the boundary test**
   `codelet/graph/tests/no_napi_dependency.rs`. Mirror the pattern from
   `codelet/agent-loop/tests/no_napi_dependency.rs`:

   ```rust
   // RPC-092: enforce no codelet-napi / napi / napi-derive edges from codelet-graph.
   use std::process::Command;

   #[test]
   fn codelet_graph_has_no_napi_dependency() {
       let output = Command::new(env!("CARGO"))
           .args(["metadata", "--no-deps", "--format-version", "1"])
           .output()
           .expect("cargo metadata");
       let json: serde_json::Value =
           serde_json::from_slice(&output.stdout).expect("parse metadata");
       let pkg = json["packages"]
           .as_array()
           .unwrap()
           .iter()
           .find(|p| p["name"] == "codelet-graph")
           .expect("codelet-graph in workspace");
       let deps: Vec<String> = pkg["dependencies"]
           .as_array()
           .unwrap()
           .iter()
           .filter_map(|d| d["name"].as_str().map(str::to_owned))
           .collect();
       for forbidden in ["codelet-napi", "napi", "napi-derive"] {
           assert!(
               !deps.contains(&forbidden.to_owned()),
               "codelet-graph must not depend on {forbidden}; found in {deps:?}"
           );
       }
   }
   ```

2. **Run the boundary test:**
   ```bash
   cargo test -p codelet-graph --test no_napi_dependency
   ```
   Must pass.

3. **Run the graph reset integration test:**
   ```bash
   cargo test -p codelet-graph --test graph_reset_test
   ```
   Must pass (8 reset scenarios).

4. **Run the 24 downstream NAPI integration tests** to verify the
   shim works:
   ```bash
   cargo test -p codelet-napi --test ast_extraction_pipeline_test
   cargo test -p codelet-napi --test ast_query_interface_test
   cargo test -p codelet-napi --test ast_hierarchy_test
   # … all 24
   ```
   Must all pass without modification.

5. **Diff the cargo metadata** to confirm no surprises:
   ```bash
   cargo metadata --no-deps --format-version 1 > /tmp/rpc092-after.json
   # codelet-graph is a NEW node; no other crate should have lost edges
   ```

6. **Checkpoint:**
   ```
   fspec checkpoint RPC-092 rpc092-tests-green
   ```

---

## Phase 5 — Coverage + validate + done

1. **Link coverage** for every scenario in the feature file
   `spec/features/codelet-graph-crate-lift.feature` (created in Phase 1
   of the standard fspec workflow):

   ```
   fspec link-coverage codelet-graph-crate-lift \
     --scenario "codelet-graph crate exists as a workspace member" \
     --testFile codelet/graph/tests/no_napi_dependency.rs \
     --testLines "5-30" \
     --implFile codelet/graph/Cargo.toml \
     --implLines "1-40"
   ```
   (repeat per scenario)

2. **Validate the workspace:**
   ```bash
   cargo check --workspace
   cargo clippy --workspace -- -D warnings
   ```

3. **Update RPC-092 status:**
   ```
   fspec update-work-unit-status RPC-092 validating
   # … after coverage shown 100%:
   fspec update-work-unit-status RPC-092 done
   ```

4. **Unblock RPC-072 Phase B** by removing the block edge:
   ```
   # The blockedBy edge is removed automatically when RPC-092 is done.
   # Verify RPC-072 board state shows it ready to move back into `testing`.
   fspec show-work-unit RPC-072
   ```

---

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Path rewrite typo breaks a single extractor | Phase 3 verification step `cargo check --workspace` will catch it before any test runs. |
| `include_str!` path mismatch on a schema | The 4 sites are explicit and listed in `architecture.md` § Path rewrites. |
| One of the 24 NAPI tests touches a private graph module item | Audit before Phase 3 by `grep -h 'crate::graph::\|graph::' codelet/napi/tests/ast_*_test.rs | sort -u`. If anything is `pub(crate)`-only, promote it to `pub` in the lifted module BEFORE moving. |
| `graph_reset_tests.rs` uses `crate::graph::*` paths that need rewriting | Phase 3 step (3) explicitly addresses this. |
| nanograph version drift between `codelet-napi` and `codelet-graph` | Both pin `nanograph = "1.0.0"` literally. Promote to workspace if needed in a follow-up. |
| Boundary test fails because `codelet-providers` transitively depends on `codelet-napi` | Verify the dep graph at end of Phase 1. If a transitive edge exists, separate issue — file a follow-up card. From inspection today, this is clean. |

---

## Rollback plan

Any phase can be rolled back via:
```
fspec restore-checkpoint RPC-092 <previous-checkpoint-name>
```

Phases produce checkpoints at:
- `rpc092-pre-lift-baseline` (Phase 0)
- `rpc092-scaffold` (Phase 1)
- `rpc092-schemas-moved` (Phase 2)
- `rpc092-sources-moved` (Phase 3)
- `rpc092-tests-green` (Phase 4)

If Phase 3 explodes mid-move (which it shouldn't — it's a single
commit), restore to `rpc092-schemas-moved` and re-attempt.
