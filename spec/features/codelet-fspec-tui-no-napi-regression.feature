@done
@session-management
@codelet
@rust
@infrastructure
@napi
@RPC-044
Feature: codelet-fspec-tui no codelet-napi dependency regression guard

  # Architectural invariant enforced by RPC-044 Phase 5: the fspec-tui library —
  # which the fspec binary embeds — MUST NOT transitively depend on
  # codelet-napi. The AgentView reaches the SessionManager through the
  # FspecBackend trait, never through the NAPI bridge. This regression test
  # mirrors rust/sessions/tests/skeleton_invariants.rs and
  # rust/rpc-embedded/tests/rpc_006_source_shape.rs.
  Background: User Story
    As a Rust developer maintaining the codelet workspace
    I want a regression test that asserts codelet-fspec-tui does not transitively depend on codelet-napi (neither in cargo metadata nor in source imports)
    So that the forbidden fspec-tui → napi dependency arrow can never silently reappear

  @rule:fspec_tui_no_napi_test
  @test
  @architecture
  Scenario: codelet-fspec-tui has no transitive codelet-napi dependency and no codelet_napi imports
    Given the RPC-044 changes are applied to the codelet workspace
    When I run `cargo test -p codelet-fspec-tui --test no_napi_dependency`
    Then the command exits with code 0
    And the test parses `cargo metadata` output for codelet-fspec-tui
    And the resulting transitive package set does not contain the package name `codelet-napi`
    And no `.rs` file under `rust/fspec-tui/src/` contains the substring `codelet_napi`
