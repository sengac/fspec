@done
@high
@rust
@workspace
@lift
@migration
@codelet
@RPC-092
Feature: Lift codelet/napi/src/graph/ (52 files, ~15,013 LOC) into a NAPI-free codelet-graph workspace crate — verbatim relocation, NO cut-downs

  """
  Source-of-truth: codelet/napi/src/graph/ (52 .rs files, ~15,013 LOC, zero direct NAPI coupling). Lift target: codelet/graph/ as a peer workspace crate. See spec/attachments/RPC-092/inventory.md for the complete file-by-file LOC table.
  Why this card was carved (instead of inlining into RPC-072 Phase B): the body port at codelet/napi/src/agent_loop.rs:299-1456 calls graph_search_handler::create_handler() and register_deep_search_handler(...), both of which transitively pull crate::graph::*. Inlining a 15,013-LOC mechanical relocation into the 1,170-LOC body port would produce a single ~30,000-LOC commit with no checkpoint between stages — context-window-exhausting and impossible to bisect. RPC-092 is the discrete, atomic relocation step. See spec/attachments/RPC-092/root-cause-analysis.md.
  Why a stub-handler / BridgeHandlers trait was REJECTED: a NoopBridgeHandlers impl would silently swallow GraphSearch + DeepSearch tool calls — exactly the kind of functional gap that put the original 203-line agent_loop stub in the broken state RPC-072 is now repairing. RPC-072 Rule [5] mandates zero functional cut-downs. The same invariant applies here.
  NAPI shim contract: replace the deleted codelet/napi/src/graph/ directory with a single file codelet/napi/src/graph.rs containing only `pub use codelet_graph::*;`. This preserves the crate::graph::* import surface for the 24 codelet/napi/tests/ast_*_test.rs integration tests so they pass with ZERO test-source modification.
  Mechanical path rewrites are the ONLY edits permitted: (a) `crate::graph::*` → `crate::*` because the lifted crate's root IS the old `crate::graph::` namespace; (b) `include_str!("../../schemas/...")` → `include_str!("../schemas/...")` at the 4 call sites in registry.rs, ast_dispatch.rs, and learnings_dispatch.rs because files move from src/graph/*.rs to src/*.rs while schemas move from napi/schemas/ to graph/schemas/. See spec/attachments/RPC-092/architecture.md § Path rewrites for the full table.
  5-phase implementation: Phase 0 (pre-flight + baseline checkpoint), Phase 1 (scaffold empty codelet-graph crate), Phase 2 (move 4 schemas), Phase 3 (single-commit git mv of all 52 .rs files + path rewrites + NAPI shim), Phase 4 (boundary test + 24 NAPI tests still green), Phase 5 (coverage linking + validate + done). Each phase produces a named checkpoint for rollback. See spec/attachments/RPC-092/implementation-plan.md.
  Test pyramid (4 tiers): Tier 1 = NEW codelet/graph/tests/no_napi_dependency.rs boundary guard (THE acceptance bar — mirrors RPC-067 pattern). Tier 2 = lifted graph_reset_test.rs (8 tests). Tier 3 = preserved codelet/napi/tests/ast_*_test.rs suite (24 tests, must pass through shim with zero modification). Tier 4 = `cargo clippy --workspace -- -D warnings` clean. See spec/attachments/RPC-092/test-plan.md for the full Done definition.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. codelet-graph MUST exist as a peer workspace member under codelet/graph/ — added to codelet/Cargo.toml [workspace] members and [workspace.dependencies]
  #   2. codelet-graph MUST have zero dependency edges to codelet-napi, napi, or napi-derive — enforced by a new codelet/graph/tests/no_napi_dependency.rs that shells out to cargo metadata and asserts on the dependencies array
  #   3. All 52 .rs files (16 root + 33 ast_pipeline + 3 ast_call_chain) MUST be moved verbatim with git mv to preserve blame history — no edits to logic during the move, only mechanical path rewrites (crate::graph::* → crate::*) and include_str! relative-path adjustments
  #   4. All 4 bundled schemas (ast-code.pg, ast-queries.gq, learnings.pg, learnings-queries.gq) MUST move from codelet/napi/schemas/ to codelet/graph/schemas/ — they are compiled into the binary via include_str! and have no runtime path dependency
  #   5. codelet/napi/src/graph/ directory MUST be replaced by a single file codelet/napi/src/graph.rs containing only `pub use codelet_graph::*;` — this preserves the existing crate::graph::* import surface for the 24 codelet/napi/tests/ast_*_test.rs integration tests
  #   6. All 24 existing codelet/napi/tests/ast_*_test.rs integration tests MUST pass post-lift with zero modification to test source — the shim re-export is the contract
  #   7. The existing in-module graph_reset_tests.rs (8 #[cfg(test)] tests) MUST be lifted to codelet/graph/tests/graph_reset_test.rs as a standalone integration test using codelet_graph::* imports — same 8 scenarios, same assertions
  #   8. codelet-agent-loop MUST be able to add `codelet-graph = { workspace = true }` to its [dependencies] and lift deep_search_handler.rs + graph_search_handler.rs without re-introducing a codelet-napi edge — verified by codelet/agent-loop/tests/no_napi_dependency.rs still passing
  #   9. cargo clippy --workspace -- -D warnings MUST be clean post-lift — workspace lints (no unwrap_used, no expect_used, no panic, etc.) apply transitively to the new crate
  #   10. Manual smoke test (./fspec graph-index .; ./fspec graph-search-stats; ./fspec graph-search 'Auth') MUST produce identical output to the pre-lift baseline — proves no behavioural drift from path rewrites
  #
  # EXAMPLES:
  #   1. After lift, `cargo metadata --no-deps --format-version 1 | jq '.packages[] | select(.name == "codelet-graph").dependencies[].name'` lists nanograph, codelet-providers, ast-grep-core, ast-grep-language, ignore, chrono, globset, serde, serde_json, sha2, tracing, lazy_static, uuid, arrow-array, arrow-schema, rig-core — and NEVER codelet-napi, napi, or napi-derive
  #   2. Running `cargo test -p codelet-graph --test no_napi_dependency` outputs `test codelet_graph_has_no_napi_dependency ... ok` and the test binary completes with status 0 — the maintainer sees a green pass
  #   3. Running `cargo test -p codelet-graph --test graph_reset_test` reports 8 passed / 0 failed — every scenario from the old in-module graph_reset_tests.rs still proves the reset semantics post-lift
  #   4. Running `cargo test -p codelet-napi --tests` produces zero regressions vs the pre-lift baseline — all 24 ast_*_test.rs integration tests still pass through the thin `pub use codelet_graph::*;` shim with zero source modification
  #   5. After the lift, the maintainer adds `codelet-graph = { workspace = true }` to codelet/agent-loop/Cargo.toml and `pub mod deep_search_handler;` to codelet/agent-loop/src/lib.rs — `cargo check -p codelet-agent-loop` succeeds AND `cargo test -p codelet-agent-loop --test no_napi_dependency` still passes
  #   6. Running `cargo clippy --workspace -- -D warnings` exits 0 — the workspace-scoped lints (no unwrap, no expect, no panic, no todo) propagate cleanly to codelet-graph because no logic was modified
  #   7. After lift, running `./fspec graph-index .` against a project (e.g., the fspec repo itself) emits the same byte-for-byte output as it did pre-lift — same entity counts, same edge counts, same elapsed-time order-of-magnitude — proving no behavioural drift from path rewrites
  #   8. Running `git log --follow codelet/graph/src/ast_pipeline/ast_ts_extractor.rs` shows the full history dating back to the file's original creation under codelet/napi/src/graph/ast_pipeline/ — git mv preserved blame, satisfying the verbatim-lift rule
  #   9. Once RPC-092 is marked done, `fspec show-work-unit RPC-072` shows blockedBy with RPC-092 no longer present (or marked as resolved), and the maintainer can run `fspec update-work-unit-status RPC-072 testing` without an ACDD violation
  #   10. Pre-lift baseline checkpoint `rpc092-pre-lift-baseline` exists in `fspec list-checkpoints RPC-092` — at any point during the lift, the maintainer can run `fspec restore-checkpoint RPC-092 rpc092-pre-lift-baseline` and recover the workspace to a known-good state
  #
  # ========================================

  Background: User Story
    As a rust workspace maintainer
    I want to lift codelet/napi/src/graph/ verbatim into a NAPI-free codelet-graph crate
    So that codelet-agent-loop can host deep_search_handler + graph_search_handler and unblock RPC-072 Phase B without re-introducing a codelet-napi dependency edge

  Scenario: codelet-graph exists as a workspace member with the expected dependency surface
    Given the lift has been completed against the workspace at codelet/Cargo.toml
    When I run `cargo metadata --no-deps --format-version 1` and inspect the dependencies of the codelet-graph package
    Then the dependencies array lists nanograph, codelet-providers, ast-grep-core, ast-grep-language, ignore, chrono, globset, serde, serde_json, sha2, tracing, lazy_static, uuid, arrow-array, arrow-schema, and rig-core
    And the dependencies array contains zero entries for codelet-napi, napi, or napi-derive

  Scenario: codelet-graph boundary guard test passes
    Given the lift has been completed and codelet/graph/tests/no_napi_dependency.rs is in place
    When I run `cargo test -p codelet-graph --test no_napi_dependency`
    Then the test binary prints `test codelet_graph_has_no_napi_dependency ... ok`
    And the test binary exits with status 0

  Scenario: Lifted graph reset integration tests still prove the reset semantics
    Given the old in-module codelet/napi/src/graph/graph_reset_tests.rs has been lifted to codelet/graph/tests/graph_reset_test.rs as a standalone integration test
    When I run `cargo test -p codelet-graph --test graph_reset_test`
    Then the test runner reports 8 passed and 0 failed
    And every scenario from the original graph_reset_tests.rs is exercised against the lifted codelet_graph modules

  Scenario: All 24 existing NAPI ast_*_test.rs integration tests still pass through the shim
    Given the codelet/napi/src/graph/ directory has been replaced by codelet/napi/src/graph.rs containing only `pub use codelet_graph::*;`
    And no source under codelet/napi/tests/ has been modified
    When I run `cargo test -p codelet-napi --tests`
    Then all 24 ast_*_test.rs integration tests pass with zero regressions vs the pre-lift baseline

  Scenario: codelet-agent-loop can depend on codelet-graph without re-introducing a NAPI edge
    Given the lift has been completed
    When I add `codelet-graph = { workspace = true }` to codelet/agent-loop/Cargo.toml and `pub mod deep_search_handler;` to codelet/agent-loop/src/lib.rs
    Then `cargo check -p codelet-agent-loop` succeeds
    And `cargo test -p codelet-agent-loop --test no_napi_dependency` still passes

  Scenario: Workspace clippy gate is clean after the lift
    Given the lift has been completed and only mechanical path rewrites have been applied to the lifted files
    When I run `cargo clippy --workspace -- -D warnings`
    Then the command exits with status 0
    And no workspace lint (unwrap_used, expect_used, panic, todo, dbg_macro) reports a violation in the codelet-graph crate

  Scenario: Manual graph-index smoke test shows no behavioural drift
    Given a pre-lift baseline output of `./fspec graph-index .` has been captured against the fspec repository
    When the lift has been completed and I run `./fspec graph-index .` against the same repository at the same revision
    Then the entity counts match the pre-lift baseline
    And the edge counts match the pre-lift baseline
    And the elapsed-time order-of-magnitude matches the pre-lift baseline

  Scenario: Git blame history is preserved through git mv
    Given all 52 .rs files have been moved with `git mv` rather than copy-delete
    When I run `git log --follow codelet/graph/src/ast_pipeline/ast_ts_extractor.rs`
    Then the log shows the file's full history dating back to its original creation under codelet/napi/src/graph/ast_pipeline/
    And the same property holds for every other file under codelet/graph/src/

  Scenario: RPC-072 is unblocked once RPC-092 is marked done
    Given RPC-092 has been moved to status `done` and the blocks edge to RPC-072 has fired
    When I run `fspec show-work-unit RPC-072`
    Then the blockedBy field no longer references RPC-092 as an open dependency
    And running `fspec update-work-unit-status RPC-072 testing` succeeds without an ACDD violation

  Scenario: Pre-lift baseline checkpoint enables rollback at any phase
    Given the maintainer created the checkpoint `rpc092-pre-lift-baseline` during Phase 0
    When I run `fspec list-checkpoints RPC-092`
    Then the checkpoint `rpc092-pre-lift-baseline` appears in the list
    And running `fspec restore-checkpoint RPC-092 rpc092-pre-lift-baseline` restores the workspace to a known-good pre-lift state
