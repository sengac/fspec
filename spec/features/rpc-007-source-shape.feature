@done
@source-shape
@rust
@rpc
@regression
@RPC-007
Feature: RPC-007 source-shape regression invariants
  """
  Static source-level invariants that protect the architectural decisions of
  RPC-007: (a) codelet/rpc must NOT depend on codelet-napi (preserves the
  RPC-006 source-shape rule); (b) the five lifted types — SessionId,
  SessionInfo, SessionStatus, StreamChunk, LogRecord — must each have exactly
  ONE definition site, located in codelet/rpc-types. codelet/napi must
  re-export each via the existing #[cfg_attr(feature = "napi", napi(...))]
  pattern so the TS shape sees zero change.
  """

  Background: User Story
    As a developer maintaining the dual-transport RPC architecture
    I want a source-level regression test that fails when rpc starts depending on napi or when a lifted type gets duplicated
    So that the trait+handle and shared NAPI contract invariants cannot be silently broken by future cards

  @rpc
  @source-shape
  @regression
  Scenario: Source-shape regression: rpc → napi remains forbidden and the five new types are defined exactly once
    Given the workspace contains codelet/rpc, codelet/rpc-types, codelet/rpc-server, codelet/rpc-embedded, codelet/napi, and codelet/core
    When cargo metadata is queried for codelet/rpc/Cargo.toml dependencies
    Then no dependency named codelet-napi is present
    When ast-grep searches the workspace for definitions of StreamChunk, SessionInfo, SessionStatus, SessionId, and LogRecord
    Then each type has exactly one definition site, located in codelet/rpc-types
    And codelet/napi re-exports each type via the existing #[cfg_attr(feature = "napi", napi(...))] pattern
