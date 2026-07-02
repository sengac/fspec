@done
@RPC-338
@rpc
@model-selection
@parity
@wip
Feature: Model selector profile cross-transport parity

  # Work unit: RPC-338. Both transports must return identical profile_name and
  # is_unreachable values (codelet/fspec-tui/tests/rpc338_cross_transport_parity.rs).
  Background: User Story
    As a codelet TUI user
    I want list_providers() to behave identically over both transports
    So that the model selector renders the same profile and reachability state everywhere

  @websocket
  @integration
  Scenario: Both transports return identical profile and reachability fields
    Given a provider set containing a local profile section and a cloud provider
    When list_providers() is called over the in-process transport
    And list_providers() is called over the websocket transport
    Then both responses contain the same profile_name values for every provider
    And both responses contain the same is_unreachable values for every provider
