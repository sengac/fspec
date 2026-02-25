@done
@TUI-073
@tui-component
@model-selector
Feature: Create ModelSelectorScreen component
  """
  ARCHITECTURE:

  ModelSelectorScreen is an ORCHESTRATOR component that composes:

  ┌─────────────────────────────────────────────────────────────┐
  │ ModelSelectorScreen (new file: src/tui/components/)        │
  │   ├── useModelSelectorState()  ← state & operations        │
  │   ├── useInput()               ← keyboard handling         │
  │   └── <ModelSelectorView />    ← presentation only         │
  └─────────────────────────────────────────────────────────────┘

  RESPONSIBILITIES:
  - ModelSelectorScreen: Owns useInput handler, translates keys to hook operations
  - useModelSelectorState (TUI-072): State, navigation, model loading, filtering logic
  - ModelSelectorView: Renders UI, receives all data via props, NO useInput

  REQUIRED CHANGES:
  1. CREATE: src/tui/components/ModelSelectorScreen.tsx (~150 lines)
  2. MODIFY: ModelSelectorView.tsx - remove useInput (lines 250-363), add callback props
  3. DELETE from AgentView.tsx:
  - State declarations (lines 1045-1059, ~15 vars)
  - Keyboard handling (lines 6641-6808, ~170 lines)
  - Inline rendering (lines 7384-7548, ~165 lines)

  DEPENDENCIES:
  - TUI-072 (DONE): useModelSelectorState hook
  - TUI-076 (DONE): Consolidated types in src/tui/types/provider.ts

  INTEGRATION POINTS:
  - AgentView calls: <ModelSelectorScreen onSelectModel={...} onClose={...} onSwitchToSettings={...} />
  - AgentView manages: showModelSelector boolean, receives ModelSelection from callback
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ModelSelectorScreen must use useModelSelectorState hook for all state management
  #   2. ModelSelectorScreen must handle ALL keyboard input via useInput (Escape, Tab, arrows, Enter, r, /)
  #   3. ModelSelectorScreen must call onClose callback when user presses Escape (and no filter active)
  #   4. ModelSelectorScreen must call onSwitchToSettings callback when user presses Tab
  #   5. ModelSelectorScreen must call onSelectModel callback with ModelSelection when user selects a model
  #   6. ModelSelectorView must be modified to remove its useInput handler (becomes purely presentational)
  #
  # EXAMPLES:
  #   1. User presses Down arrow → hook's navigateDown() is called, selection moves to next item
  #   2. User presses Escape with no filter → onClose callback is invoked
  #   3. User presses Escape with active filter → filter is cleared, onClose NOT called
  #   4. User presses Tab → onSwitchToSettings callback is invoked
  #   5. User presses Enter on model item → onSelectModel called with ModelSelection, then onClose
  #   6. User presses Enter on section header → toggleSectionExpansion is called
  #   7. User presses 'r' key → refreshModels() is called
  #   8. User presses '/' key → filter mode is activated
  #   9. User presses Left arrow → current section collapses
  #  10. User presses Right arrow → current section expands
  #  11. Filter mode: typing characters appends to filter, backspace removes last char
  #  12. Filter mode: Enter exits filter mode, Escape clears filter and exits mode
  #
  # ========================================
  Background: User Story
    As a developer
    I want to use a ModelSelectorScreen component that handles all model selection input
    So that AgentView.tsx is reduced by ~400 lines and keyboard handling is encapsulated

  # ===========================================
  # NAVIGATION SCENARIOS
  # ===========================================
  @navigation
  Scenario: Navigate down in model list
    Given the ModelSelectorScreen is rendered with provider sections
    When the user presses the Down arrow key
    Then the hook's navigateDown function should be called
    And the selection should move to the next item in the list

  @navigation
  Scenario: Navigate up in model list
    Given the ModelSelectorScreen is rendered with provider sections
    And the selection is not on the first item
    When the user presses the Up arrow key
    Then the hook's navigateUp function should be called
    And the selection should move to the previous item in the list

  @navigation
  Scenario: Collapse section with Left arrow
    Given the ModelSelectorScreen is rendered with provider sections
    And the current section is expanded
    When the user presses the Left arrow key
    Then the section should collapse
    And the selection should move to the section header

  @navigation
  Scenario: Expand section with Right arrow
    Given the ModelSelectorScreen is rendered with provider sections
    And the current section is collapsed
    When the user presses the Right arrow key
    Then the section should expand

  # ===========================================
  # CLOSE BEHAVIOR SCENARIOS
  # ===========================================
  Scenario: Close screen with Escape when no filter is active
    Given the ModelSelectorScreen is rendered
    And no filter is currently active
    When the user presses the Escape key
    Then the onClose callback should be invoked

  Scenario: Clear filter with Escape when filter is active
    Given the ModelSelectorScreen is rendered
    And a filter is currently active
    When the user presses the Escape key
    Then the filter should be cleared
    And the onClose callback should NOT be invoked

  # ===========================================
  # SCREEN SWITCHING SCENARIOS
  # ===========================================
  Scenario: Switch to provider settings with Tab
    Given the ModelSelectorScreen is rendered
    When the user presses the Tab key
    Then the onSwitchToSettings callback should be invoked

  # ===========================================
  # MODEL SELECTION SCENARIOS
  # ===========================================
  Scenario: Select a model with Enter
    Given the ModelSelectorScreen is rendered with provider sections
    And the selection is on a model item
    When the user presses the Enter key
    Then the onSelectModel callback should be invoked with a ModelSelection object
    And the onClose callback should be invoked

  Scenario: Toggle section expansion with Enter on section header
    Given the ModelSelectorScreen is rendered with provider sections
    And the selection is on a section header
    When the user presses the Enter key
    Then the toggleSectionExpansion function should be called
    And the section should expand or collapse

  # ===========================================
  # FILTER MODE SCENARIOS
  # ===========================================
  @filtering
  Scenario: Enter filter mode with slash key
    Given the ModelSelectorScreen is rendered
    And filter mode is not active
    When the user presses the "/" key
    Then filter mode should be activated

  @filtering
  Scenario: Type characters in filter mode
    Given the ModelSelectorScreen is rendered
    And filter mode is active
    When the user types printable characters
    Then the characters should be appended to the filter string

  @filtering
  Scenario: Delete characters in filter mode with backspace
    Given the ModelSelectorScreen is rendered
    And filter mode is active with text "clau"
    When the user presses the Backspace key
    Then the filter should become "cla"

  @filtering
  Scenario: Exit filter mode with Enter
    Given the ModelSelectorScreen is rendered
    And filter mode is active
    When the user presses the Enter key
    Then filter mode should be deactivated
    And the filter text should be preserved

  @filtering
  Scenario: Clear filter and exit filter mode with Escape
    Given the ModelSelectorScreen is rendered
    And filter mode is active with text "clau"
    When the user presses the Escape key
    Then the filter should be cleared
    And filter mode should be deactivated

  # ===========================================
  # UTILITY KEY SCENARIOS
  # ===========================================
  Scenario: Refresh models with r key
    Given the ModelSelectorScreen is rendered
    When the user presses the "r" key
    Then the refreshModels function should be called

  # ===========================================
  # COMPONENT STRUCTURE SCENARIOS
  # ===========================================
  @state-management
  Scenario: ModelSelectorScreen uses useModelSelectorState hook
    Given the ModelSelectorScreen component is rendered
    Then it should initialize the useModelSelectorState hook
    And all state should be managed through the hook

  Scenario: ModelSelectorView is purely presentational
    Given the ModelSelectorView component exists
    Then it should NOT contain any useInput handlers
    And it should receive all data and callbacks via props

  Scenario: Auto-expand section containing current model on open
    Given the ModelSelectorScreen is rendered with a currentModelId prop
    When models are loaded
    Then the section containing that model should be auto-expanded

