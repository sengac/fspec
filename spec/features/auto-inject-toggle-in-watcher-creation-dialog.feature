@watcher-management
@codelet
@WATCH-021
Feature: Auto-Inject Toggle in Watcher Creation Dialog

  """
  Requires WATCH-020's NAPI binding to accept auto_inject parameter in session_create_watcher or session_set_role
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. WatcherCreateView adds an 'Auto-inject' toggle field between Brief and Create button in the focus order
  #   2. Auto-inject toggle defaults to enabled (true) - matching WATCH-020's default behavior for autonomous injection
  #   3. Left/Right arrow keys toggle the auto-inject value when the field is focused (consistent with Authority selector pattern)
  #   4. The onCreate callback signature must be extended to include autoInject: boolean as the fifth parameter
  #   5. AgentView's handleWatcherCreate must pass autoInject to the NAPI layer when creating the watcher session
  #   6. Auto-inject toggle displays as '[●] Enabled' (green) or '[ ] Disabled' (gray) with hint text '(←/→ to toggle)' when focused
  #   7. When auto-inject is disabled, the watcher UI will show pending injections for manual review (actual behavior implemented in WATCH-020)
  #
  # EXAMPLES:
  #   1. User opens watcher creation dialog → sees Auto-inject field defaulted to Enabled → leaves it enabled → creates watcher → watcher will automatically inject when it detects issues
  #   2. User opens watcher creation dialog → tabs to Auto-inject field → presses Right arrow to disable → sees '[  ] Disabled' in gray → creates watcher → watcher will show pending injections for manual review
  #   3. User is in Brief field → presses Tab → focus moves to Auto-inject field → presses Tab again → focus moves to Create button (auto-inject is between brief and create in focus order)
  #   4. User tabs to Auto-inject field when enabled → sees 'Auto-inject:' label in cyan, '[●] Enabled' in green with blue highlight, and '(←/→ to toggle)' hint
  #
  # ========================================

  Background: User Story
    As a user creating a new watcher
    I want to toggle whether the watcher automatically injects messages or requires manual review
    So that I can choose between fully autonomous watchers or watchers that ask before injecting

  Scenario: Auto-inject defaults to enabled
    Given the user opens the watcher creation dialog
    Then the Auto-inject field should show '[●] Enabled' in green


  Scenario: User disables auto-inject with arrow keys
    Given the watcher creation dialog is open
    And the Auto-inject field is focused and shows Enabled
    When the user presses the Right arrow key
    Then the Auto-inject field should show '[ ] Disabled' in gray


  Scenario: Tab navigation includes auto-inject field
    Given the watcher creation dialog is open
    And the Brief field is focused
    When the user presses Tab
    Then the Auto-inject field should be focused
    When the user presses Tab again
    Then the Create button should be focused


  Scenario: Auto-inject field shows focus styling and hint
    Given the watcher creation dialog is open
    When the user tabs to the Auto-inject field
    Then the 'Auto-inject:' label should be cyan
    And the toggle should have a blue background highlight
    And the hint '(←/→ to toggle)' should be visible


  Scenario: Creating watcher passes auto-inject setting
    Given the watcher creation dialog is open
    And the user has entered a valid role name
    And the user has disabled auto-inject
    When the user presses Enter to create the watcher
    Then onCreate should be called with autoInject set to false

