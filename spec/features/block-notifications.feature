@BLOCK-006
Feature: Block Notifications

  """
  Block Notifications: When AI action is blocked, emit a notification event to TUI showing 'AI was blocked from {action} - {reason}'. Use existing NotificationDialog or toast pattern. Notifications appear briefly then auto-dismiss, allowing user to see what AI attempted.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Notify user in TUI when AI is blocked
  #   2. Notifications show what action was attempted and why it was blocked
  #   3. Notifications auto-dismiss after brief display
  #   4. User can see what AI attempted even when they didn't initiate
  #
  # EXAMPLES:
  #   1. Block notification: AI writes 'src/auth.ts' in testing stage → User sees toast: 'AI was blocked from writing src/auth.ts - Cannot write impl files in testing stage'
  #   2. Command block notification: AI runs 'git checkout' → User sees toast: 'AI was blocked from git checkout - Use git switch instead'
  #
  # ========================================

  Background: Block Notifications Enabled
    Given the user has fspec TUI running with notifications enabled

  # ====================
  # NOTIFICATION DISPLAY
  # ====================

  Scenario: Notify user when AI command is blocked
    Given a blocklist rule exists blocking "git checkout" with reason "Use git switch instead"
    When the AI runs "git checkout main" via Bash
    Then the command should be blocked
    And the user should see a notification "AI was blocked from git checkout - Use git switch instead"
    And the notification should auto-dismiss

  Scenario: Notify user when AI file write is blocked by stage permissions
    Given the current work unit is in "testing" stage
    And "testing" stage only allows writing to "spec" and "test" categories
    When the AI tries to write to "src/auth.ts"
    Then the write should be blocked
    And the user should see a notification "AI was blocked from writing src/auth.ts - Cannot write impl files in testing stage"
    And the notification should auto-dismiss
