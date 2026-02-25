@done
@provider-settings
@model-selector
@integration
@tui
@TUI-075
Feature: Integrate screen components into AgentView

  """
  ModelSelectorScreen manages its own state via useModelSelectorState hook
  ProviderSettingsScreen manages its own state via useProviderSettingsState hook
  AgentView only coordinates screen visibility (showModelSelector, showSettingsTab) and receives model selection callback
  Model data (providerSections, currentModel, modelsInitialized) must be in a shared Zustand store
  useModelSelectorState hook reads/writes to the shared store, not local useState
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ModelSelectorScreen must render when showModelSelector state is true
  #   2. ProviderSettingsScreen must render when showSettingsTab state is true
  #   3. Tab key must switch between model selector and provider settings screens
  #   4. Escape key must close both screens and return to main view
  #   5. All dead code referencing undefined state setters must be removed
  #   6. AgentView must only keep showModelSelector, showSettingsTab, and currentModel state for screen coordination
  #   7. AgentView must NOT have providerSections state - useModelSelectorState hook owns this data
  #   8. AgentView must NOT have modelsInitialized state - useModelSelectorState hook owns this data
  #   9. AgentView must NOT call modelsListAll - useModelSelectorState hook handles model loading
  #   10. AgentView must receive currentModel from ModelSelectorScreen callback, not from its own state loading
  #
  # EXAMPLES:
  #   1. User types /model command → ModelSelectorScreen renders with current model highlighted
  #   2. User types /provider command → ProviderSettingsScreen renders with provider list
  #   3. User presses Tab in ModelSelectorScreen → switches to ProviderSettingsScreen
  #   4. User presses Tab in ProviderSettingsScreen → switches to ModelSelectorScreen
  #   5. User selects model in ModelSelectorScreen → currentModel updates in AgentView and session receives new model
  #   6. User presses Escape in either screen → returns to main AgentView
  #   7. AgentView init → does NOT call modelsListAll → models load when ModelSelectorScreen mounts
  #   8. User never opens /model command → models are never loaded → no wasted API calls
  #
  # ========================================

  Background: User Story
    As a developer
    I want to integrate ModelSelectorScreen and ProviderSettingsScreen into AgentView
    So that AgentView is reduced by 800+ lines and screen logic is properly encapsulated

  @smoke
  Scenario: Open model selector screen via /model command
    Given I am in the main AgentView
    When I type the "/model" command
    Then the ModelSelectorScreen should be displayed
    And the current model should be highlighted

  @smoke
  Scenario: Open provider settings screen via /provider command
    Given I am in the main AgentView
    When I type the "/provider" command
    Then the ProviderSettingsScreen should be displayed
    And the provider list should be visible

  Scenario: Switch from model selector to provider settings via Tab
    Given I have the ModelSelectorScreen open
    When I press the Tab key
    Then the ModelSelectorScreen should close
    And the ProviderSettingsScreen should be displayed

  Scenario: Switch from provider settings to model selector via Tab
    Given I have the ProviderSettingsScreen open
    When I press the Tab key
    Then the ProviderSettingsScreen should close
    And the ModelSelectorScreen should be displayed

  @critical
  Scenario: Model selection updates session
    Given I have the ModelSelectorScreen open
    And I have an active session
    When I select a different model
    Then the ModelSelectorScreen should close
    And the session should use the newly selected model

  Scenario: Close model selector screen via Escape
    Given I have the ModelSelectorScreen open
    When I press the Escape key
    Then the ModelSelectorScreen should close
    And the main AgentView should be displayed

  Scenario: Close provider settings screen via Escape
    Given I have the ProviderSettingsScreen open
    When I press the Escape key
    Then the ProviderSettingsScreen should close
    And the main AgentView should be displayed

  @critical
  Scenario: AgentView does not have duplicate model state
    Given the AgentView component source code
    Then it should NOT contain "useState<ProviderSection[]>"
    And it should NOT contain "modelsListAll"
    And it should NOT contain "setProviderSections"
    And it should NOT contain "modelsInitialized" state declaration

  @critical
  Scenario: Model data is loaded lazily when model selector opens
    Given I am in the main AgentView
    And no models have been loaded yet
    When I type the "/model" command
    Then models should be loaded from the shared store
    And the ModelSelectorScreen should display the loaded models

  @critical
  Scenario: Shared store provides model data to both AgentView and ModelSelectorScreen
    Given the model store contains provider sections
    When AgentView needs to display current model info
    Then it reads from the shared store
    And when ModelSelectorScreen renders the model list
    Then it also reads from the same shared store
