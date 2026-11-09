@done
@BUG-446
@tui
@test-maintenance
@rust
Feature: The REFAC-019 contract suite runs against the fallible MCP catalog
  """
  codelet_rpc::mcp_dispatch::methods() returns Result<Vec<Method>,
  CatalogError> (BUG-413), but rust/fspec-tui/tests/refac019_contracts.rs
  indexed it as a Vec, so the whole target failed to compile and none of the
  REFAC-019 contract checks ran. The MCP registration scenario now propagates
  the catalog's Result.
  Tests: rust/fspec-tui/tests/refac019_contracts.rs.

  Merged into preserve-contracts-while-decomposing-native-board-integration-surfaces (REFAC-019) by COV-075 (product owner, 2026-11-05): its scenario restated 'MCP mappings equal the resolved service methods exactly' on the same test and was removed here.
  """

  Background: User Story
    As a developer relying on the REFAC-019 contract suite
    I want to have refac019_contracts compile and run
    So that the TUI backend and RPC surface contracts are actually checked
