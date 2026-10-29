@BOARD-024
@board
@rpc
@tui
@rust
@integration
@phase-1
@critical
Feature: Board card details canonical backend
  """
  BOARD-024 loads an entire raw workUnits map record from the explicitly attached daemon workspace, not the board cache, CLI projection or client current directory. get_work_unit_details returns Result<String,String> with JSON envelope id/workspace/workUnit for bincode compatibility. Unknown, historical and legacy fields remain intact. Every request reads current canonical data and missing workspace, card, store, malformed data and read failures are explicit errors. Existing dynamic MCP dispatch drives real embedded and WebSocket RPC clients before typed API calls can compile. Typed FspecBackend assertions will follow the first approved API implementation; this stage does not claim that coverage already exists.
  """

  Background: User Story
    As a developer inspecting board cards
    I want complete current records and honest canonical read errors from the daemon workspace
    So that details do not silently substitute summaries stale data or client-local files

  Scenario: Canonical details preserve the complete record and daemon workspace across both transports
    Given a daemon workspace with a complete canonical card including historical unknown and legacy fields
    When the canonical details RPC is invoked through embedded and WebSocket clients
    Then both clients receive the same complete id workspace workUnit JSON envelope without projection or truncation

  Scenario: Canonical details read updated storage instead of cached board snapshots
    Given a canonical card and an earlier captured board summary snapshot
    When canonical storage changes and each transport requests the same card again
    Then both responses contain the new canonical record while the original snapshot remains unchanged

  Scenario: Canonical loader failures remain explicit daemon errors on both transports
    Given isolated fixtures with missing workspace card or store unreadable store malformed JSON invalid store or empty identifier
    When each failure is requested through embedded and WebSocket canonical details dispatch
    Then every request returns an explicit contextual daemon error rather than unknown method transport failure empty data or stale success
