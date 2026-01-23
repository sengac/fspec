@watcher
@tui
@WATCH-009
Feature: Watcher Creation Dialog UI

  """
  Add showWatcherCreateDialog useState and form state (watcherName, watcherAuthority, watcherModel, watcherBrief, createDialogFocus) to AgentView.tsx
  Replace TODO in N key handler (line 4741) with setShowWatcherCreateDialog(true)
  Import sessionCreateWatcher from codelet-napi
  Add handleWatcherCreate callback that calls sessionCreateWatcher then sessionSetRole, refreshes watcher list via handleWatcherMode
  Add watcher creation dialog render block in watcher overlay section, shown when showWatcherCreateDialog is true
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. N key in Watcher Management overlay opens Watcher Creation dialog
  #   2. Dialog has input field for watcher role name (e.g., 'Code Reviewer', 'Security Auditor')
  #   3. Dialog has authority selector with two options: Peer and Supervisor
  #   4. Dialog has model selector populated from available provider models
  #   5. Dialog has optional brief/description textarea for watcher role context
  #   6. Tab key cycles focus between input fields: name → authority → model → brief → name
  #   7. Enter key on Create button calls sessionCreateWatcher(parentId, model, project, name) then sessionSetRole(watcherId, name, brief, authority)
  #   8. Esc key closes dialog and returns to Watcher Management overlay
  #   9. After successful creation, dialog closes, overlay refreshes to show new watcher in list
  #   10. Role name field is required - Create button disabled if empty
  #
  # EXAMPLES:
  #   1. User presses N in overlay → dialog opens with empty name field focused, authority=Peer (default), model=current session model
  #   2. User types 'Code Reviewer' in name field, presses Tab → focus moves to authority selector
  #   3. User presses ←/→ on authority selector → toggles between Peer and Supervisor
  #   4. User fills name='Code Reviewer', authority=Peer, model=claude-sonnet-4, brief='Reviews code changes' → presses Enter on Create → watcher created, dialog closes, overlay shows new watcher
  #   5. User presses Esc in dialog → dialog closes, returns to Watcher Management overlay unchanged
  #   6. User with empty name field presses Enter on Create → nothing happens (Create disabled), name field shows required indicator
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to create a new watcher via a dialog from the Watcher Management overlay
    So that I can spawn AI agents to observe my parent session

  
  Scenario: Open watcher creation dialog with N key
    Given the Watcher Management overlay is open
    When the user presses the N key
    Then the Watcher Creation dialog should open
    And the name input field should be empty and focused
    And the authority should be set to "Peer" by default
    And the model should be set to the current session model

  
  Scenario: Tab through dialog fields
    Given the Watcher Creation dialog is open
    And the name field is focused
    When the user presses Tab
    Then the authority selector should be focused
    When the user presses Tab
    Then the model selector should be focused
    When the user presses Tab
    Then the brief textarea should be focused
    When the user presses Tab
    Then the Create button should be focused
    When the user presses Tab
    Then the name field should be focused again

  
  Scenario: Toggle authority with arrow keys
    Given the Watcher Creation dialog is open
    And the authority selector is focused with value "Peer"
    When the user presses the right arrow key
    Then the authority should change to "Supervisor"
    When the user presses the left arrow key
    Then the authority should change to "Peer"

  
  Scenario: Create watcher successfully
    Given the Watcher Creation dialog is open
    And the user has entered name "Code Reviewer"
    And the user has selected authority "Peer"
    And the user has selected model "claude-sonnet-4"
    And the user has entered brief "Reviews code changes"
    When the user presses Enter on the Create button
    Then sessionCreateWatcher should be called with the parent session ID and model
    And sessionSetRole should be called with the new watcher ID, name, brief, and authority
    And the dialog should close
    And the Watcher Management overlay should show the new watcher "Code Reviewer"

  
  Scenario: Cancel watcher creation with Escape
    Given the Watcher Creation dialog is open
    And the user has entered some data in the fields
    When the user presses Escape
    Then the dialog should close
    And the Watcher Management overlay should be visible
    And the watcher list should be unchanged

  
  Scenario: Create button disabled when name is empty
    Given the Watcher Creation dialog is open
    And the name field is empty
    When the user presses Enter on the Create button
    Then no watcher should be created
    And the name field should show a required indicator
