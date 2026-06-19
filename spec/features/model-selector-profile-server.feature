@done
@RPC-338 @providers @model-selection @ts-parity @wip
Feature: Model selector profile server data source

  # Work unit: RPC-338. Server-side list_providers() population — ports the
  # decision logic of TS profileSectionBuilder.ts (reachability probe +
  # MODEL-004 custom-models override). codelet/sessions/src/profile_sections.rs.

  Background: User Story
    As a codelet TUI user
    I want the server to report local-server profile sections and their reachability
    So that profile sections and unreachable markers appear in the model selector

  @server
  Scenario: A cloud provider reports no profile and is reachable
    Given the server builds the provider list
    And a cloud provider "anthropic" with credentials is registered
    When list_providers() returns the provider list
    Then the "anthropic" entry has profile_name None
    And the "anthropic" entry has is_unreachable false

  @server
  Scenario: A reachable local-server profile is reported as a profile section
    Given the server builds the provider list
    And a local-server "openai" profile named "my-profile" whose /v1/models probe succeeds
    When list_providers() returns the provider list
    Then a provider entry has profile_name Some("my-profile")
    And that entry has display_name "openai: my-profile"
    And that entry has is_unreachable false

  @server
  Scenario: An unreachable local-server profile with no custom models is marked unreachable
    Given the server builds the provider list
    And a local-server "openai" profile named "down-profile" whose /v1/models probe fails
    And that profile has no custom models
    When list_providers() returns the provider list
    Then the "down-profile" entry has is_unreachable true
    And the "down-profile" entry has profile_name Some("down-profile")
    And the "down-profile" entry has an empty models list
    And the "down-profile" entry has display_name "openai: down-profile" with no embedded "(unreachable)" text
    And the "down-profile" entry is still present in the list

  @server
  Scenario: A failed-probe profile with custom models is not marked unreachable
    Given the server builds the provider list
    And a local-server "openai" profile named "custom-profile" whose /v1/models probe fails
    And that profile has at least one custom model
    When list_providers() returns the provider list
    Then the "custom-profile" entry has is_unreachable false
    And the "custom-profile" entry still has profile_name Some("custom-profile")
    And the "custom-profile" entry lists its custom models
