@REFAC-027
@rpc
@refactor
@code-quality
Feature: Remove the legacy codelet-napi crate
  """
  rust/napi was the NAPI-RS adapter for the retired Ink/React TypeScript TUI; nothing depends on
  it. It is removed from the workspace together with the rpc-types `napi` feature and every test,
  scenario and doc reference that asserted napi behaviour. The separation it once motivated is
  carried by the crate graph: engine (codelet-core, codelet-sessions, codelet-agent-loop,
  codelet-tools, codelet-providers), service (codelet-rpc), transports (codelet-rpc-server,
  codelet-rpc-embedded) and client (codelet-fspec-tui), checked via `cargo metadata`.
  """

  Background: User Story
    As an fspec maintainer
    I want the unused NAPI adapter gone
    So that the workspace only carries the Rust daemon, transports and client it ships

  Scenario: The workspace has no napi crate
    Given the rust workspace manifest and every member manifest
    When the workspace members and their dependencies are listed
    Then no member is codelet-napi and no manifest depends on codelet-napi, napi or napi-derive
    And the rust/napi directory does not exist

  Scenario: No code reads napi sources or gates on a napi feature
    Given every Rust source and test file in the workspace
    When their non-comment code is scanned
    Then none reads a rust/napi path or uses feature = "napi"

  Scenario: The daemon and client separation still holds
    Given the workspace dependency graph
    When the arrows between engine, service, transport and client crates are inspected
    Then engine crates depend on no service, transport or client crate
    And the RPC service depends on no transport or client crate
    And the transports depend on no client crate
