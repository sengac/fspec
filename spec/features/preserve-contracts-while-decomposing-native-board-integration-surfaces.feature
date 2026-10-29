@REFAC-019
@tui
@rpc
@integration
@code-quality
@phase-1
Feature: Preserve contracts while decomposing native board integration surfaces
  """
  User approved the proper preparatory refactor before BOARD-024 integration.
  Preserve existing public imports, full member signatures and default bodies,
  independent item ordering and generated tarpc wire ordering. Whole-item
  accumulator macros assemble contiguous original members before derive,
  async_trait and tarpc attributes expand; includes contain complete Rust items.
  Keep expansion in the original logical module to preserve super paths and
  generated public types. Resolve only actual includes and actual macro chains,
  rejecting cycles, disconnected definitions and broken forwarding. Build schema
  tracking includes reachable declaration inputs; no protocol redesign occurs.
  New schema hashes may differ conservatively while the running daemon remains
  unchanged. Preserve Done BUG-283 and all unrelated checkout work.
  """

  Background: User Story
    As a maintainer of native board and RPC integration
    I want to decompose oversized modules without changing their contracts
    So that board dialogs can evolve without breaking existing consumers

  Scenario: Ordered public and implementation contracts retain their baseline
    Given frozen contracts for Action backend and RPC declarations and implementations
    When their actual source composition is resolved
    Then ordered members signatures attributes and bodies match the baseline

  Scenario: Backend consumers retain object safety and existing defaults
    Given an existing external backend implementation and both native backend types
    When they are consumed through the original public trait object
    Then existing consumer compilation and default behavior remain intact

  Scenario: Decomposed integration source remains below the module ceiling
    Given the six approved integration entrypoints and their actual included sources
    When their maintained Rust source sizes are inspected
    Then every targeted entrypoint and extracted source is below 300 lines

  Scenario: MCP mappings equal the resolved service methods exactly
    Given the actual composed RPC service declaration and typed MCP registry
    When their method names are compared
    Then names match exactly without duplicate registrations and schemas remain strict
