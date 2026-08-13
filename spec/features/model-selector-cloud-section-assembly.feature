@model-selection
@wip
@ts-parity
@model-selector
@tui
@PROV-127
Feature: Cloud section assembly drops empty cloud sections (TS parity)
  """
  PROV-127 unit layer for the section-assembly rule. The pure helper
  rust/sessions/src/profile_sections.rs::retain_populated_cloud_sections
  drops cloud sections (profile_name == None) whose model list is empty, while
  never dropping local-server profile sections (profile_name == Some). Mirrors
  TS cloudSectionBuilder.ts filter(s => s.hasCredentials) +
  modelInitializationService.ts filter(s => s.models.length > 0). Wired into
  handle_impl.rs::list_providers() before local profile sections are appended.
  """

  Background: User Story
    As a developer using the /model selector
    I want the provider-list assembly to drop empty cloud sections while keeping local profiles
    So that the picker matches the TypeScript reference without dropping reachable-but-empty profiles

  Scenario: A cloud provider with no models is dropped from the provider list
    Given a canonical cloud provider that resolves to an empty model list
    When the provider list is assembled
    Then that cloud provider section is not present in the list

  Scenario: A cloud provider with one or more models is kept unchanged
    Given a canonical cloud provider that resolves to a non-empty model list
    When the provider list is assembled
    Then that cloud provider section is present with its models intact

  Scenario: Local profile sections are not affected by the empty-cloud filter
    Given a local profile section with an empty model list
    When the provider list is assembled
    Then the local profile section is still present in the list
