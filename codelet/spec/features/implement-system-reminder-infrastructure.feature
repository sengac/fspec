@critical @session @context-management @session-management @system-reminders @compaction
@CLI-012
Feature: Implement system-reminder infrastructure
  """
  Architecture notes:
  - System-reminders are Messages added to the messages Vec (user role with special content)
  - Compaction-safe: system-reminders are explicitly EXCLUDED from compaction (filtered out before summarizing)
  - Position persistence: system-reminders stay in their positions while messages around them are compacted
  - Type marker format: <!-- type:tokenStatus --> embedded in content for identification
  - Automatic deduplication via messages.retain() to remove old type, then push new one
  - Persists through compaction like anchors (stays in place indefinitely)
  - Cache-friendly: staying in place avoids prepending that would shift all positions

  Critical implementation requirements:
  - System-reminder messages use user role with content wrapped in <system-reminder> tags
  - Compaction MUST filter out system-reminders before summarizing (partition messages)
  - After compaction, system-reminders MUST be added back to messages Vec
  - add_system_reminder() MUST use messages.retain() to remove old type, then push new
  - is_system_reminder() helper MUST identify messages by type marker (<!-- type:xxx -->)

  Dependencies:
  - Session struct (src/session/mod.rs) - messages Vec contains system-reminders
  - SystemReminderType enum (src/session/system_reminders.rs)
  - Compaction system (src/agent/compaction.rs) - needs partition logic to exclude system-reminders
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. System reminders MUST be Messages in the messages array (user role with special content)
  #   2. System reminders MUST be explicitly EXCLUDED from compaction (filtered out before summarizing)
  #   3. System reminders MUST stay in their positions during compaction (messages around them are compacted)
  #   4. Each type MUST have exactly one instance (deduplication via retain+push pattern)
  #   5. System reminders MUST use type markers for identification (<!-- type:xxx -->)
  #
  # EXAMPLES:
  #   1. Add tokenStatus reminder - it's added to messages array at current position (end)
  #   2. Add environment reminder, then tokenStatus reminder - both exist in messages array at their positions
  #   3. Compaction filters out system-reminders, summarizes other messages, then adds system-reminders back - they persist
  #   4. Update tokenStatus reminder - messages.retain() removes old tokenStatus, then push new one (deduplication)
  #
  # ========================================
  Background: User Story
    As a LLM agent in interactive session
    I want to see dynamic context via system-reminders
    So that I can respond to queries about token status, environment, and documentation without fabricating data

  Scenario: Add system-reminder to messages array
    Given I have a Session with empty messages Vec
    When I call add_system_reminder(tokenStatus, "50% tokens used")
    Then the messages Vec should contain 1 message
    And the message should be user role with content wrapped in <system-reminder> tags
    And the message content should include type marker "<!-- type:tokenStatus -->"
    And the message content should include "50% tokens used"

  Scenario: Multiple reminder types coexist in messages array
    Given I have a Session with empty messages Vec
    When I call add_system_reminder(environment, "Platform: linux")
    And I call add_system_reminder(tokenStatus, "60% tokens used")
    Then the messages Vec should contain 2 system-reminder messages
    And the first system-reminder should have type marker "<!-- type:environment -->"
    And the second system-reminder should have type marker "<!-- type:tokenStatus -->"


  Scenario: Identify system-reminder messages
    Given I have a message with system-reminder tags and type marker
    When I check if it's a system-reminder using is_system_reminder helper
    Then it should be identified as a system-reminder
    And regular user messages should NOT be identified as system-reminders
    And assistant messages should NOT be identified as system-reminders

  Scenario: Partition messages for compaction
    Given I have a Session with mixed messages: user messages, system-reminders, and assistant messages
    When I partition messages using partition_for_compaction helper
    Then system-reminders should be in separate partition
    And compactable messages should be in separate partition
    And all system-reminders should have type markers
    And no compactable messages should be system-reminders

  Scenario: Reconstruct messages after compaction preserves reminders
    Given I have messages partitioned into system-reminders and compactable
    When I simulate compaction by replacing compactable with summary
    And I reconstruct: summary + system-reminders + recent messages
    Then the reconstructed messages should contain the summary
    And the reconstructed messages should contain system-reminders
    And system-reminder content should be unchanged
    And the message count should be reduced (compaction happened)

  Scenario: Deduplication via retain and push pattern
    Given I have a Session with messages: [user_msg_1, system-reminder(tokenStatus, "50% used")]
    When I call add_system_reminder(tokenStatus, "60% used")
    Then the messages Vec should contain exactly 1 tokenStatus system-reminder
    And the tokenStatus system-reminder content should be "60% used"
    And the old "50% used" tokenStatus should be removed
