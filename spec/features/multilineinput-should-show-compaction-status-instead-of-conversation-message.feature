@done
@feature-management
@input
@tui
@UX-002
Feature: MultiLineInput should show compaction status instead of conversation message

  """
  Remove duplicate UI-generated compaction success message at AgentView.tsx:7456 (retry flow)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. MultiLineInput must show compaction status in its placeholder area when isCompacting=true
  #   2. Conversation area must NOT show '[Compacting context...]' status messages during compaction
  #   3. MultiLineInput must remain interactive during compaction to handle ESC key for cancellation
  #   4. Input typing should be disabled during compaction but keyboard navigation should still work
  #   5. This story fixes the incomplete implementation of PERF-002 Example 2 and 4 which specified progress in input area but was wrongly implemented in thinking area
  #
  # EXAMPLES:
  #   1. User types /compact, sees 'Analyzing anchors... 15/32 turns' as input placeholder, NOT in conversation
  #   2. User presses ESC during compaction, compaction is interrupted, input placeholder returns to normal
  #   3. User tries typing during compaction, characters are not captured/displayed, but input area still shows compaction progress
  #   4. Conversation history is clean without '[Compacting context...]' messages appearing between user messages
  #   5. User types /compact, input placeholder changes from 'Type a message...' to 'Compacting: analyzing anchors... 15/32 turns' and then to 'Compacting: generating summary...'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should ESC key cancel/interrupt compaction, or should compaction be non-interruptible from the UI?
  #   A: Compaction should be non-interruptible. Input area should not respond to ESC during compaction and should not display any hints about ESC cancellation.
  #
  #   Q: When compaction completes, should we show a success message in the input area briefly, or immediately return to normal placeholder?
  #   A: No success message. Return immediately to the next state as determined by Rust-controlled state updates. No UI-side status management.
  #
  #   Q: Should input area show detailed progress like 'Analyzing anchors... 15/32 turns' or simplified 'Compacting...' message?
  #   A: Show detailed progress: 'Compacting: analyzing anchors... 15/32 turns' format in the input placeholder area. This was specified in PERF-002 but incorrectly implemented only in thinking area.
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to see compaction status in the input area itself
    So that I get clear feedback without polluting the conversation with status messages

  Scenario: Input placeholder shows compaction progress instead of conversation message
    Given I have a conversation with multiple turns
    When I type "/compact" and press Enter
    Then the input placeholder should show "Compacting: analyzing anchors... 15/32 turns"
    And the conversation history should NOT contain "[Compacting context...]" messages
    And the input area should remain visible and responsive

  Scenario: Input placeholder shows detailed compaction phases
    Given I have started a compaction process
    When the compaction progresses through phases
    Then the input placeholder should show "Compacting: analyzing anchors... 15/32 turns"
    And then it should show "Compacting: generating summary..."
    And no compaction status should appear in the conversation area

  Scenario: Input area blocks typing but shows progress during compaction
    Given compaction is in progress
    When I try to type characters in the input area
    Then the characters should not be captured or displayed
    And the input placeholder should continue showing compaction progress
    And I should not be able to submit messages

  Scenario: Conversation history remains clean without compaction status messages
    Given I have a clean conversation with user and AI messages
    When I run a compaction process
    Then the conversation should only contain actual user and AI messages
    And there should be no "[Compacting context...]" status messages
    And there should be no other system status messages related to compaction

  Scenario: Input area returns to normal state after compaction completes
    Given compaction is showing progress in the input placeholder
    When the compaction process completes successfully
    Then the input placeholder should immediately return to "Type a message..."
    And I should be able to type and submit messages normally
    And the conversation should show the compaction result message only
