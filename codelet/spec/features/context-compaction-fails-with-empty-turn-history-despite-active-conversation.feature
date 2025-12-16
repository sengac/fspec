@CTX-002
Feature: Context compaction fails with empty turn history despite active conversation

  """
  ARCHITECTURAL MISMATCH: Rust port uses eager turn creation (after each interaction) while TypeScript uses lazy turn creation (during compaction only). This causes extraction failures to silently fail in Rust, leaving session.turns empty.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The create_conversation_turn_from_last_interaction function must successfully extract text from rig message structures (OneOrMany<Content>) without returning None
  #   2. Conversation turns must be created and stored in session.turns vector for compaction to access them
  #   3. Error messages must be accurate and actionable - empty turn history error is misleading when messages exist but extraction fails
  #   4. SOLUTION: Follow TypeScript implementation exactly - use lazy turn creation during compaction (like convertToConversationFlow in compaction.ts) instead of eager creation after each interaction
  #
  # EXAMPLES:
  #   1. Session with 5 conversation turns, 81 messages, 800K+ tokens triggers compaction at sequence 645, 848, 1169 but fails with 'Cannot compact empty turn history'
  #   2. Debug logs show session.messages contains 81 messages with substantial content, but session.turns vector is empty because create_conversation_turn_from_last_interaction returns None
  #   3. TypeScript version uses lazy turn creation during compaction (convertToConversationFlow) with forward iteration, while Rust uses eager creation after each interaction with backward iteration
  #   4. Root cause: extract_text_from_assistant and extract_text_from_user functions fail to extract content from OneOrMany<Content> structures, returning empty strings, causing line 56 check to fail and return None
  #   5. TypeScript compaction.ts lines 99-140: convertToConversationFlow creates turns lazily during compaction using simple forward iteration through message pairs - this is what Rust should replicate
  #
  # ========================================

  Background: User Story
    As a developer using codelet Rust port
    I want to have context compaction work correctly during long conversations
    So that memory is managed efficiently and conversations don't fail with misleading errors

  Scenario: Context compaction succeeds with conversation history
    Given I have a session with 81 messages in conversation history
    And the session has accumulated 800000 tokens
    And compaction threshold has been exceeded
    When the compaction system attempts to compress the conversation
    Then conversation turns should be created successfully from message history
    And the compaction should succeed without errors
    And the effective token count should be reduced

  Scenario: Turn creation uses lazy approach during compaction
    Given I have multiple user and assistant message pairs in session history
    When the compaction system converts messages to conversation turns
    Then turns should be created using forward iteration through message pairs
    And each user-assistant pair should become a single conversation turn
    And turn creation should happen during compaction not after each interaction