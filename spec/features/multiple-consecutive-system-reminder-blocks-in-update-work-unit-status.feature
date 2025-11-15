@done
@critical
@cli
@system-reminders
@bug-fix
@VAL-004
Feature: Multiple consecutive system-reminder blocks in update-work-unit-status

  """
  The update-work-unit-status command collects multiple system reminders from helper functions (getStatusChangeReminder, getVirtualHooksReminder, getVirtualHooksCleanupReminder) that each return content wrapped in system-reminder tags. To avoid consecutive blocks, the implementation strips wrapper tags from each reminder, then wraps the combined content once using wrapInSystemReminder utility.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All system-reminder content must be in a single unified block
  #   2. Multiple reminders should be combined by unwrapping individual blocks and re-wrapping once
  #
  # EXAMPLES:
  #   1. Work unit transitions to done status with multiple applicable reminders - output should have single system-reminder block containing all reminder content
  #   2. Status change reminder, virtual hooks reminder, and cleanup reminder all triggered - should combine into one block, not three separate blocks
  #
  # ========================================

  Background: User Story
    As a developer using fspec CLI
    I want to see single unified system-reminder blocks
    So that I don't get confused by multiple consecutive reminder blocks

  Scenario: Multiple reminders combined into single block
    Given multiple system reminders are applicable for a status transition
    And each reminder is individually wrapped in system-reminder tags
    When the update-work-unit-status command processes the reminders
    Then all reminder content should be in a single system-reminder block
    And there should be no consecutive system-reminder blocks in the output

  Scenario: Status transition to done with multiple applicable reminders
    Given a work unit is transitioning to done status
    And the status change reminder is applicable
    And the virtual hooks cleanup reminder is applicable
    And the quality check review reminder is applicable
    When the status update is executed
    Then a single system-reminder block should contain all three reminders
    And the reminders should be separated by blank lines within the block
