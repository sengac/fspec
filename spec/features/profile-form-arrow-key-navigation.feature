@TUI-084
Feature: Profile form uses Tab for field navigation instead of Arrow keys Up/Down

  """
  Changes profileFormModeHandler.ts to use key.downArrow/key.upArrow instead of key.tab. Updates ProviderSettingsPanel.tsx footer hint text. Updates parent feature spec provider-settings-screen.feature scenario accordingly.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Profile form fields must be navigated with ↑/↓ arrow keys, NOT Tab/Shift+Tab
  #   2. Footer hint text must show '↑/↓: switch field | Enter: save | Esc: cancel' instead of 'Tab: next field | Shift+Tab: prev'
  #   3. Tab key in profile form mode must be ignored (no action) - it must NOT navigate fields
  #   4. Arrow Down moves to next field (index+1), Arrow Up moves to previous field (index-1)
  #
  # EXAMPLES:
  #   1. User presses ↓ on Base URL field → focus moves to API Key field
  #   2. User presses ↑ on API Key field → focus moves back to Base URL field
  #   3. User presses Tab in profile form → nothing happens (Tab does NOT navigate fields)
  #   4. Footer text shows '↑/↓: switch field | Enter: save | Esc: cancel'
  #
  # ========================================

  Background: User Story
    As a developer
    I want to navigate profile form fields with Arrow keys Up/Down
    So that the navigation is consistent with standard form UX and doesn't conflict with Tab switching between screens

  Scenario: Navigate to next field with Down arrow
    Given the user is in profile form mode on the Base URL field
    When the user presses the Down arrow key
    Then the focus moves to the API Key field


  Scenario: Navigate to previous field with Up arrow
    Given the user is in profile form mode on the API Key field
    When the user presses the Up arrow key
    Then the focus moves back to the Base URL field


  Scenario: Tab key does not navigate form fields
    Given the user is in profile form mode on the Base URL field
    When the user presses the Tab key
    Then the focus remains on the Base URL field


  Scenario: Footer shows arrow key navigation hints
    Given the user is in profile form mode
    Then the footer shows arrow-key switch field and Enter save and Esc cancel hints

