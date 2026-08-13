@done
@RPC-338
@rpc
@model-selection
@ts-parity
@wip
Feature: Model selector profile wire types

  # Work unit: RPC-338. Example-mapping context (rules/examples/assumptions)
  # lives on the work unit: `fspec show-work-unit RPC-338`.
  # Wire-type layer: ProviderInfo (rust/rpc-types/src/lib.rs) gains
  # profile_name: Option<String> + is_unreachable: bool.
  Background: User Story
    As a codelet TUI user
    I want the provider wire type to carry profile and reachability metadata
    So that profile sections and unreachable markers can flow over both transports

  @data-model
  Scenario: ProviderInfo carries profile and reachability fields over the wire
    Given a ProviderInfo value constructed with its derived Default
    Then its profile_name field is None
    And its is_unreachable field is false
    And the field profile_name has type Option<String>
    And the field is_unreachable has type bool
