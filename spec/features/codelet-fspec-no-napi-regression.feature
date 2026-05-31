@done
@session-management
@codelet
@rust
@infrastructure
@napi
@RPC-044
Feature: codelet-fspec no codelet-napi dependency regression guard

  # Architectural invariant enforced by RPC-044 Phase 5: the fspec binary MUST
  # NOT transitively depend on the codelet-napi crate. The agent surface reaches
  # the binary through the NAPI-free codelet-sessions crate, not through the
  # NAPI bridge. This regression test mirrors the cargo-metadata + source-import
  # walk pattern in codelet/sessions/tests/skeleton_invariants.rs and
  # codelet/rpc-embedded/tests/rpc_006_source_shape.rs.
  Background: User Story
    As a Rust developer maintaining the codelet workspace
    I want a regression test that asserts codelet-fspec does not transitively depend on codelet-napi (neither in cargo metadata nor in source imports)
    So that the forbidden fspec → napi dependency arrow can never silently reappear

  @rule:fspec_no_napi_test
  @test
  @architecture
  Scenario: codelet-fspec has no transitive codelet-napi dependency and no codelet_napi imports
    Given the RPC-044 changes are applied to the codelet workspace
    When I run `cargo test -p codelet-fspec --test no_napi_dependency`
    Then the command exits with code 0
    And the test parses `cargo metadata` output for codelet-fspec
    And the resulting transitive package set does not contain the package name `codelet-napi`
    And no `.rs` file under `codelet/fspec/src/` contains the substring `codelet_napi`
