@done
@coverage-tracking
@source-shape
@regression
@testing
@rpc
@RPC-068
Feature: Final TS-frontend regression + boundary audit
  """
  codelet-sessions owns the agent loop (BackgroundSession + SessionManager) and is NAPI-free; codelet-napi is a thin #[napi] adapter that subscribes to chunks_tx::broadcast and republishes via the JS ThreadsafeFunction so TS keeps its sessionSetGlobalChunkCallback API
  The dependency-rule tests in rust/{core,sessions,rpc-types,fspec,fspec-tui}/tests/no_napi_dependency.rs each assert (a) no `use codelet_napi` substring in src/ after comment stripping and (b) no codelet-napi node in the transitive `cargo metadata` graph
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. rust/napi/src/session_manager.rs MUST be deleted
  #   2. rust/napi/src/persistence/ MUST contain only mod.rs + napi_bindings.rs (plus optional test helpers)
  #   3. GLOBAL_CHUNK_CALLBACK static and the GlobalChunkCallback unsafe Send/Sync impls MUST be deleted from executable code
  #   4. tokio::broadcast::Sender<(SessionId, StreamChunk)> MUST be wired in rust/sessions/src/{session_manager.rs, background_session.rs}
  #   5. No `.rs` file under rust/{core,rpc,rpc-types,rpc-embedded,rpc-server,fspec,fspec-tui,sessions}/src/ MAY contain `use codelet_napi` or `codelet_napi::`
  #   6. No Cargo.toml in rust/{core,rpc,rpc-types,rpc-embedded,rpc-server,fspec,fspec-tui,sessions} MAY declare an active codelet-napi dependency
  #   7. rust/napi/index.d.ts MUST NOT remove any export from the pre-RPC-030 baseline (function-surface regression)
  #   8. A markdown audit report MUST be committed at spec/attachments/RPC-068/boundary-audit-report.md so future agents can verify the invariants
  #
  # EXAMPLES:
  #   1. Running `cargo test -p codelet-core -p codelet-sessions -p codelet-rpc-types -p codelet-fspec -p codelet-fspec-tui --test no_napi_dependency` exercises all five no-`napi` dependency-rule tests (core, sessions, rpc-types, fspec, fspec-tui) and reports 10/10 passing
  #   2. Grepping for `static GLOBAL_CHUNK_CALLBACK` across rust/ returns zero matches in executable code (only comments/tests reference the name as a historical marker)
  #   3. Diffing `rust/napi/index.d.ts` exports against the pre-RPC-030 baseline `ea0ed0a0` shows 191 baseline exports preserved and 5 additive exports (countCheckpoints, getModelInfo, getWorkspaceInfo, moveWorkUnitUp, moveWorkUnitDown) — no removals
  #   4. Listing `rust/napi/src/persistence/` shows exactly `mod.rs` and `napi_bindings.rs`; all six pure-Rust persistence types (`message_envelope`, `messages`, `manifest`, `blob`, `blob_processing`, `history`) live in `rust/core/src/persistence/`
  #   5. Running `npm test` produces 4747 passing tests and the watch-024 supervisor-terminology suite reports 16/16 passing after the stale-path fix updates the test to read from the new union of `rust/sessions/src/*.rs` + `rust/napi/src/{session_bindings,agent_loop,bridges}.rs`
  #   6. A `spec/attachments/RPC-068/boundary-audit-report.md` exists, summarises every verification matrix item, and lists the precise pass/fail counts so a future agent can re-run the audit and compare
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to run the final TS-frontend regression and Rust boundary audit at the end of the RPC-030 chain
    So that I can prove the codelet-napi → codelet-sessions extraction kept the TS-facing API surface intact and the no-`napi` dependency invariants hold across `core`, `rpc`, `rpc-types`, `rpc-embedded`, `rpc-server`, `fspec`, `fspec-tui`, and `sessions`

  Scenario: Dependency-rule regression tests pass across every forbidden crate
    Given the RPC-030 chain has landed
    When I run `cargo test -p codelet-core -p codelet-sessions -p codelet-rpc-types -p codelet-fspec -p codelet-fspec-tui --test no_napi_dependency`
    Then the suite picks up the no_napi_dependency.rs target from `codelet-core`, `codelet-sessions`, `codelet-rpc-types`, `codelet-fspec`, and `codelet-fspec-tui`
    And each target reports 2 / 2 passing tests
    And the aggregate result is 10 / 10 passing tests across all five targets

  Scenario: GLOBAL_CHUNK_CALLBACK is removed from executable code
    Given the RPC-041 broadcast-replacement card has landed
    When I run `rg "static GLOBAL_CHUNK_CALLBACK" rust/` from the repository root
    Then the search returns zero matches in executable Rust code
    And the only references that remain are doc-string comments in `rust/sessions/src/*.rs` and assertion-only references inside `rust/napi/tests/global_chunk_callback_napi_test.rs` and `rust/sessions/tests/background_session_shape.rs`

  Scenario: TS-facing NAPI export surface is a strict superset of the pre-RPC-030 baseline
    Given the pre-RPC-030 baseline `rust/napi/index.d.ts` from commit `ea0ed0a0`
    When I extract the `export declare function` identifiers from the baseline and from the current `rust/napi/index.d.ts`
    Then the current export count is 196 against a baseline of 191
    And no baseline export name is missing from the current `index.d.ts`
    And the five additive exports are `countCheckpoints`, `getModelInfo`, `getWorkspaceInfo`, `moveWorkUnitUp`, and `moveWorkUnitDown`

  Scenario: codelet-napi persistence collapses to a thin adapter
    Given the RPC-031 to RPC-035 persistence lift chain has landed
    When I list `rust/napi/src/persistence/`
    Then the directory contains exactly `mod.rs` and `napi_bindings.rs`
    And `rust/core/src/persistence/` contains the lifted pure-Rust modules `message_envelope.rs`, `messages.rs`, `manifest.rs`, `blob.rs`, `blob_processing.rs`, and `history.rs`

  Scenario: TS test suite remains green after the watch-024 path fix
    Given the RPC-030 chain has split `rust/napi/src/session_manager.rs` across `rust/sessions/src/*.rs` and `rust/napi/src/{session_bindings,agent_loop,bridges}.rs`
    When I update `src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts` to read its assertions from the union of the new file locations
    And I run `npx vitest run src/tui/__tests__/watch-024-supervisor-terminology-refactoring.test.ts`
    Then the suite reports 16 / 16 passing tests
    And a full `npm test` run reports 4747 passing tests across the repository (with the 27 remaining failures all in pre-existing Ink-rendering test files unrelated to the NAPI ↔ sessions boundary)

  Scenario: Boundary audit report is committed for future verification
    Given the audit checklist in `spec/attachments/RPC-068/final-regression-and-audit.md`
    When I run the boundary audit
    Then a markdown report is committed at `spec/attachments/RPC-068/boundary-audit-report.md`
    And the report tabulates every verification-matrix row with its observed result
    And the report records the precise pass/fail counts for the dependency-rule tests, the `index.d.ts` diff, and the TS test suite
    And the report explicitly closes out RPC-030
