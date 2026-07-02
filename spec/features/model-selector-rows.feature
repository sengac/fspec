@model-selection
@done
@ts-parity
@rust
@model-selector
@tui
@RPC-337
Feature: Model selector row rendering
  """
  Model rows display capability badges in TS order: [C] custom (yellow), [R] reasoning (magenta), [V] vision (blue), [{cw}] context window (gray); badges are dimmed/normal per selection, matching ModelSelectorView.tsx. The row whose model id matches the active session's current model shows a green '(current)' marker. The body renders a legend line '[R] Reasoning | [V] Vision | [C] Custom' inside the body region (not shell chrome). Providers load asynchronously via Action::ListProvidersLoaded and a placeholder shows until loaded.
  """

  Background: User Story
    As a fspec TUI user
    I want model rows to show capability badges, a current marker and a legend
    So that I can read each model's capabilities at a glance like the TypeScript view

  Scenario: Model rows display capability badges
    Given the model selector lists a custom model supporting reasoning and vision with a 200k context window
    When the row is rendered while unselected
    Then it shows the badges "[C]", "[R]", "[V]" and "[200k]" in that order
    And the "[C]" badge is yellow, "[R]" magenta, "[V]" blue and "[200k]" gray

  Scenario: The active session model shows a current marker
    Given the model selector lists a model whose id matches the active session model
    When the list is rendered
    Then that model row shows a green "(current)" marker

  Scenario: The body renders the capability legend
    Given the model selector is open
    When the body is rendered
    Then a legend line "[R] Reasoning | [V] Vision | [C] Custom" appears at the bottom of the body

  Scenario: Providers still loading shows a placeholder
    Given the model selector has opened but providers have not loaded
    When the body is rendered
    Then it shows the "No providers available" placeholder
    And the placeholder is replaced once the provider list arrives
