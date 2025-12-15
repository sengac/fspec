@CLI-013
Feature: Fix system-reminder replacement to preserve prompt cache prefix

  """
  Modifies add_system_reminder() to append without removing existing reminders. Adds supersession marker to new reminders. Updates partition_for_compaction() to extract only the latest reminder of each type. Integrates with Session.compact_messages() for proper reconstruction.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When adding a system-reminder of the same type as an existing one, the old reminder must NOT be removed from the message array
  #   2. New system-reminders are always appended to the end of the message array to preserve prompt cache prefix
  #   3. When a replacement reminder is added, it must contain a supersession marker indicating it replaces an earlier reminder of the same type
  #   4. During compaction, only the LAST (most recent) reminder of each type is preserved
  #   5. After compaction, system-reminders are placed at the START of messages (before summary) to establish new stable prefix
  #
  # EXAMPLES:
  #   1. Given [msg1, msg2, tokenStatus_v1], when add_system_reminder(TokenStatus, 'new'), then result is [msg1, msg2, tokenStatus_v1, tokenStatus_v2_with_supersession_marker] - old reminder stays in place
  #   2. Given [msg1, tokenStatus_v1, msg2, tokenStatus_v2], when partition_for_compaction(), then only tokenStatus_v2 (the latest) is extracted for preservation
  #   3. Given messages with 2 tokenStatus reminders, when compact_messages(), then result starts with [tokenStatus_v2, summary, ...] - only latest reminder preserved at start
  #   4. Supersession marker format: 'This supersedes earlier [type] reminder' appended to the reminder content
  #
  # ========================================

  Background: User Story
    As a AI agent system
    I want to add replacement system-reminders without breaking prompt cache
    So that the LLM prompt cache remains valid until compaction occurs

  # Scenario 1: Old reminder stays in place when adding replacement
  Scenario: Adding replacement reminder preserves old reminder in place
    Given a message array with [msg1, msg2, tokenStatus_v1]
    When I call add_system_reminder with TokenStatus type and new content
    Then the result should be [msg1, msg2, tokenStatus_v1, tokenStatus_v2]
    And tokenStatus_v2 should contain a supersession marker
    And the message prefix [msg1, msg2, tokenStatus_v1] should be unchanged

  # Scenario 2: Partition extracts only latest reminder of each type
  Scenario: Partition for compaction extracts only the latest reminder per type
    Given a message array with [msg1, tokenStatus_v1, msg2, tokenStatus_v2]
    When I call partition_for_compaction
    Then only tokenStatus_v2 should be in the system-reminders partition
    And tokenStatus_v1 should be in the compactable messages partition

  # Scenario 3: After compaction only latest reminder at start
  Scenario: Compaction preserves only latest reminder at start of messages
    Given a session with messages containing two tokenStatus reminders
    And the session has conversation turns to compact
    When I call compact_messages
    Then the result should start with [tokenStatus_v2, summary, ...]
    And tokenStatus_v1 should not be present in the messages

  # Scenario 4: Supersession marker format
  Scenario: Supersession marker indicates reminder supersedes earlier one
    Given a message array with an existing Environment reminder
    When I add a new Environment reminder with updated content
    Then the new reminder content should contain "This supersedes earlier environment reminder"
    And the old reminder should remain unchanged in its original position