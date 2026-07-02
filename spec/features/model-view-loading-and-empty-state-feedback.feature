@done
@model-selector
@tui
@PROV-104
@critical
@component
@feature-group
Feature: Model view loading and empty state feedback
  """
  ModelSelectorView tracks a `loaded: bool` (false until set_providers folds a list_providers result). render_body (rows_render.rs) must consult this flag: when !loaded paint a distinct loading indicator; when loaded && rows empty paint an explicit no-models empty state. Previously both cases painted the single EMPTY_PLACEHOLDER ('No providers available'), making loading and empty indistinguishable. Verified at render level (tests_loading_empty.rs) for determinism; the live keyboard-nav e2e path is covered separately in model-selector-keyboard-navigation-e2e.
  """

  Background: User Story
    As a fspec TUI user opening the /model view
    I want to see a distinct loading indicator while providers load and an explicit empty state when none exist
    So that I am never left staring at a silently inert, ambiguous blank list

  Scenario: Opening /model before providers load shows a loading state not a blank inert list
    Given the fspec binary is launched and a Work Agent is open
    When I submit "/model" and the provider list has not yet finished loading
    Then the view shows a visible loading indicator rather than a blank list

  Scenario: Opening /model with no models shows an explicit empty state
    Given the fspec binary is launched with FSPEC_USER_DIR pointing at a temp config with no profiles and no provider credentials
    When the provider list has finished loading with no selectable models
    Then the view shows an explicit no-models empty state instead of appearing to ignore arrow keys
    And I open a Work Agent and submit "/model"
