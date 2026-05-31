@done
@session-management
@codelet
@rust
@infrastructure
@napi
@RPC-044
Feature: codelet-sessions no codelet-napi dependency regression guard

  # Architectural invariant enforced by RPC-044 Phase 5: the codelet-sessions
  # crate — which RPC-044 wires into the fspec binary — MUST NOT transitively
  # depend on codelet-napi. The existing
  # codelet/sessions/tests/skeleton_invariants.rs::scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi
  # already enforces the cargo-metadata half of this invariant (RPC-038). This
  # test adds the symmetric source-import scan so the forbidden arrow stays
  # absent in source as well as in the resolved graph.
  Background: User Story
    As a Rust developer maintaining the codelet workspace
    I want a regression test that asserts codelet-sessions does not transitively depend on codelet-napi (neither in cargo metadata nor in source imports)
    So that the forbidden sessions → napi dependency arrow can never silently reappear

  @rule:sessions_no_napi_test
  @test
  @architecture
  Scenario: codelet-sessions has no transitive codelet-napi dependency and no codelet_napi imports
    Given the RPC-044 changes are applied to the codelet workspace
    When I run `cargo test -p codelet-sessions --test no_napi_dependency`
    Then the command exits with code 0
    And the test parses `cargo metadata` output for codelet-sessions
    And the resulting transitive package set does not contain the package name `codelet-napi`
    And no `.rs` file under `codelet/sessions/src/` contains the substring `codelet_napi`
