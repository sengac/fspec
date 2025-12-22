@agent-modal
@tui
@high
@NAPI-005
Feature: Manual Compaction Command

  """
  Integration: AgentModal.tsx handleSubmit intercepts /compact before session.prompt() (pattern from /debug lines 254-271). Dependency: CodeletSession in codelet-napi/src/session.rs exposes compact() method. Dependency: compact() calls execute_compaction from codelet/cli/src/interactive_helpers.rs. Type: CompactionResult contains metrics (originalTokens, compactedTokens, compressionRatio, turnsSummarized, turnsKept). Display: Result shown as tool role message (yellow) in conversation. Update: Token tracker in header refreshes immediately after compaction.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The command must show progress feedback during compaction
  #   2. After compaction completes, the session must continue seamlessly without requiring user to resend their last message
  #   3. Token counts must be updated after manual compaction to reflect the reduced context size
  #   4. AgentModal.tsx handleSubmit must intercept /compact input BEFORE calling session.prompt() (same pattern as /debug at lines 254-271)
  #   5. CodeletSession in codelet-napi/src/session.rs must expose a compact() method that calls execute_compaction from cli/src/interactive_helpers.rs
  #   6. compact() must return CompactionResult with metrics (originalTokens, compactedTokens, compressionRatio, turnsSummarized, turnsKept)
  #   7. Compaction result message must be displayed in conversation area as a tool role message (yellow text)
  #   8. Token tracker display in header must update immediately after successful compaction
  #   9. If conversation is empty, display 'Nothing to compact - no messages yet' and return early
  #
  # EXAMPLES:
  #   1. After /compact completes, user can immediately type a new message and get a response using the compacted context
  #   2. User types /compact during a long session with 150k tokens, context is reduced to ~40k tokens, compression ratio is displayed
  #   3. If compaction fails (e.g., API error), user sees error message and context remains unchanged
  #   4. User types /compact in AgentModal, sees 'Compacting context...' message, then 'Context compacted: 150000→40000 tokens, 73% compression, summarized 12 turns, kept 3 turns'
  #   5. User types /compact with no messages in session, sees 'Nothing to compact - no messages yet' and input returns to normal
  #   6. User types /compact, compaction fails due to API error, sees 'Compaction failed: [error message]' and context remains unchanged
  #   7. After successful /compact, token display in header updates from '150000↓ 5000↑' to '40000↓ 0↑' immediately
  #   8. User types /compact, conversation continues seamlessly - user can immediately type next message without resending anything
  #
  # ========================================

  Background: User Story
    As a fspec TUI user
    I want to trigger context compaction with /compact command in AgentModal
    So that I can reduce context window usage without leaving the TUI

  Scenario: Successful manual compaction with compression feedback
    Given I am in AgentModal with a conversation that has approximately 150k tokens
    When I type '/compact' and press Enter
    Then I see 'Compacting context...' message in the conversation area
    And I see a result message showing original tokens, compacted tokens, and compression percentage
    And the token display in the header updates to reflect the reduced context size


  Scenario: Empty session shows nothing to compact
    Given I am in AgentModal with no messages in the conversation
    When I type '/compact' and press Enter
    Then I see 'Nothing to compact - no messages yet' in the conversation area
    And the input field returns to normal and I can type my next message


  Scenario: Compaction failure preserves context
    Given I am in AgentModal with an active conversation
    When I type '/compact' and press Enter
    Then I see 'Compaction failed: [error message]' in the conversation area
    And the compaction API returns an error
    And my conversation context remains unchanged

