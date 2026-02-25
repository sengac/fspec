@TUI-074
Feature: Create ProviderSettingsScreen component

  """
  ProviderSettingsScreen composes useProviderSettingsState hook with useInput keyboard handler and ProviderSettingsPanel presentation component - follows orchestrator pattern established by ModelSelectorScreen (TUI-073)
  Hook mode types must be mapped to panel mode types: edit-api-key→edit-api-key, create-profile/edit-profile→profile-form, delete-profile→delete-confirm, list→list
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderSettingsScreen must use useProviderSettingsState hook for all state management
  #   2. ProviderSettingsScreen must handle ALL keyboard input via useInput - AgentView must not handle provider settings input
  #   3. Component must render ProviderSettingsPanel (existing presentation component) with props mapped from hook state
  #   4. Props interface: width, height, onClose, onSwitchToModels callbacks
  #   5. Delete confirmation mode: 'y' confirms delete, 'n' or Escape cancels
  #   6. API key edit mode: Escape cancels, Enter saves, backspace deletes char, printable chars append
  #   7. Profile form mode: Tab navigates fields, Shift+Tab goes back, Escape cancels, Enter saves
  #   8. Filter mode: Escape clears and exits, Enter exits keeping filter, backspace/chars edit filter
  #   9. List mode: arrows navigate, Enter expands/edits, 'e' edits, 'n' new profile, 'd' deletes, 't' tests, 'r' refreshes
  #   10. Tab key in list mode calls onSwitchToModels callback
  #   11. Escape in list mode with no active filter calls onClose callback
  #   12. Hook mode types (edit-api-key, create-profile, edit-profile, delete-profile) must be mapped to SettingsPanelMode for rendering
  #
  # EXAMPLES:
  #   1. User presses Tab in list mode → onSwitchToModels callback is invoked
  #   2. User presses Escape with no active filter → onClose callback is invoked
  #   3. User presses Escape with active filter → filter cleared, screen NOT closed
  #   4. User presses 'y' in delete-profile mode → removeProfile called, mode returns to list
  #   5. User presses Enter in edit-api-key mode with non-empty key → saveApiKey called, mode returns to list
  #   6. User presses Tab in profile form → formFieldIndex increments (0→1→2→3)
  #   7. User presses Down arrow in list mode → selectedIndex increments, scroll adjusts if needed
  #   8. User presses '/' in list mode → isFilterMode becomes true, filter input active
  #   9. User presses 't' on provider item → testConnection called, testResult displayed
  #   10. User presses Enter on provider item → toggleProviderExpansion called, provider expands
  #
  # ========================================

  Background: User Story
    As a developer
    I want to use ProviderSettingsScreen as an orchestrator component
    So that keyboard input handling for provider settings is encapsulated and AgentView.tsx is ~300 lines smaller

  # ========================================
  # LIST MODE - NAVIGATION & CALLBACKS
  # ========================================

  @keyboard @list-mode
  Scenario: Switch to model selector with Tab key
    Given ProviderSettingsScreen is rendered in list mode
    When the user presses the Tab key
    Then the onSwitchToModels callback is invoked

  @keyboard @list-mode
  Scenario: Close screen with Escape when no filter is active
    Given ProviderSettingsScreen is rendered in list mode
    And no filter is active
    When the user presses the Escape key
    Then the onClose callback is invoked

  @keyboard @list-mode
  Scenario: Clear filter with Escape when filter is active
    Given ProviderSettingsScreen is rendered in list mode
    And a filter "anth" is active
    When the user presses the Escape key
    Then the filter is cleared
    And the onClose callback is NOT invoked

  @keyboard @list-mode
  Scenario: Navigate down in provider list
    Given ProviderSettingsScreen is rendered in list mode
    And the selected index is 0
    When the user presses the Down arrow key
    Then the selected index increments to 1

  @keyboard @list-mode
  Scenario: Enter filter mode with slash key
    Given ProviderSettingsScreen is rendered in list mode
    When the user presses the "/" key
    Then isFilterMode becomes true
    And the filter input is active

  @keyboard @list-mode
  Scenario: Expand provider section with Enter
    Given ProviderSettingsScreen is rendered in list mode
    And the selection is on a provider item
    When the user presses the Enter key
    Then toggleProviderExpansion is called

  @keyboard @list-mode
  Scenario: Test connection with t key
    Given ProviderSettingsScreen is rendered in list mode
    And the selection is on a provider item
    When the user presses the "t" key
    Then testConnection is called for the provider

  @keyboard @list-mode
  Scenario: Refresh providers with r key
    Given ProviderSettingsScreen is rendered in list mode
    When the user presses the "r" key
    Then the providers are reloaded

  # ========================================
  # DELETE CONFIRMATION MODE
  # ========================================

  @keyboard @delete-mode
  Scenario: Confirm profile deletion with y key
    Given ProviderSettingsScreen is in delete-profile mode for profile "my-server"
    When the user presses the "y" key
    Then removeProfile is called with the profile name
    And mode returns to list

  @keyboard @delete-mode
  Scenario: Cancel profile deletion with n key
    Given ProviderSettingsScreen is in delete-profile mode for profile "my-server"
    When the user presses the "n" key
    Then removeProfile is NOT called
    And mode returns to list

  # ========================================
  # API KEY EDIT MODE
  # ========================================

  @keyboard @api-key-mode
  Scenario: Save API key with Enter
    Given ProviderSettingsScreen is in edit-api-key mode
    And the editing API key is "sk-12345"
    When the user presses the Enter key
    Then saveApiKey is called with the key value
    And mode returns to list

  @keyboard @api-key-mode
  Scenario: Cancel API key edit with Escape
    Given ProviderSettingsScreen is in edit-api-key mode
    And the editing API key is "sk-12345"
    When the user presses the Escape key
    Then saveApiKey is NOT called
    And mode returns to list

  # ========================================
  # PROFILE FORM MODE
  # ========================================

  @keyboard @profile-form-mode
  Scenario: Navigate to next field with Tab
    Given ProviderSettingsScreen is in create-profile mode
    And formFieldIndex is 0
    When the user presses the Tab key
    Then formFieldIndex increments to 1

  @keyboard @profile-form-mode
  Scenario: Cancel profile form with Escape
    Given ProviderSettingsScreen is in create-profile mode
    When the user presses the Escape key
    Then saveProfileConfig is NOT called
    And mode returns to list

  # ========================================
  # FILTER MODE
  # ========================================

  @keyboard @filter-mode
  Scenario: Exit filter mode keeping filter with Enter
    Given ProviderSettingsScreen is in filter mode
    And the filter is "anth"
    When the user presses the Enter key
    Then isFilterMode becomes false
    And the filter remains "anth"

  @keyboard @filter-mode
  Scenario: Clear filter and exit filter mode with Escape
    Given ProviderSettingsScreen is in filter mode
    And the filter is "anth"
    When the user presses the Escape key
    Then isFilterMode becomes false
    And the filter is cleared

  # ========================================
  # COMPONENT STRUCTURE
  # ========================================

  @structure
  Scenario: ProviderSettingsScreen uses useProviderSettingsState hook
    Given ProviderSettingsScreen component is implemented
    Then it uses the useProviderSettingsState hook for state management
    And it does NOT declare its own provider/navigation state

  @structure
  Scenario: ProviderSettingsScreen renders ProviderSettingsPanel
    Given ProviderSettingsScreen component is implemented
    Then it renders ProviderSettingsPanel as its presentation layer
    And it maps hook state to panel props correctly
