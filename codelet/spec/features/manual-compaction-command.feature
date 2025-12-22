@NAPI-001
Feature: Manual Compaction Command

  """
  Integrates with repl_loop.rs slash command dispatch pattern. Calls execute_compaction() from interactive_helpers.rs which uses ContextCompactor for summarization. Debug events captured via get_debug_capture_manager() for diagnostics. Token tracker updated after compaction via session.token_tracker.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The /compact command must be available as a slash command in interactive mode
  #   2. Compaction must use the same logic as automatic compaction (summarize conversation history)
  #   3. The command must show progress feedback during compaction
  #   4. After compaction completes, session continues seamlessly without requiring user to resend message
  #   5. Token counts must be updated after manual compaction to reflect reduced context size
  #   6. If session is empty, command should inform user there is nothing to compact
  #   7. If compaction fails, context must remain unchanged and user informed of error
  #
  # EXAMPLES:
  #   1. User types /compact, sees 'Compacting context...' followed by 'Context compacted: 150000→40000 tokens, 73% compression'
  #   2. User types /compact on empty session, sees 'Nothing to compact - session is empty'
  #   3. User types /compact, compaction fails due to API error, sees 'Compaction failed: API error' and 'Context remains unchanged'
  #   4. After successful /compact, user can immediately type next message and get response using compacted context
  #   5. User types /compact with small context (under threshold), compaction still runs and shows actual compression achieved
  #   6. Debug capture records compaction.manual.start, compaction.manual.complete, and compaction.manual.failed events for diagnostics
  #
  # ========================================

  Background: User Story
    As a codelet user
    I want to manually trigger context compaction using /compact
    So that I can proactively manage context size before hitting automatic thresholds

  Scenario: Successful manual compaction with compression feedback
    Given I am in an interactive session with conversation history
    And the session has accumulated 150000 input tokens
    When I type /compact
    Then I should see "Compacting context..."
    And I should see compression results showing original and compacted token counts
    And the session token tracker should reflect the reduced context size


  Scenario: Empty session shows nothing to compact
    Given I am in an interactive session with no conversation history
    When I type /compact
    Then I should see "Nothing to compact - session is empty"


  Scenario: Compaction failure preserves context and shows error
    Given I am in an interactive session with conversation history
    And the LLM API will return an error
    When I type /compact
    Then I should see an error message containing "Compaction failed"
    And I should see "Context remains unchanged"
    And the session messages should remain unchanged


  Scenario: Session continues seamlessly after compaction
    Given I am in an interactive session with conversation history
    When I type /compact
    And the compaction completes successfully
    Then I should be able to type a new message immediately
    And the agent should respond using the compacted context


  Scenario: Small context compaction still runs
    Given I am in an interactive session with minimal conversation history
    And the session has only 5000 input tokens
    When I type /compact
    Then the compaction should still execute
    And I should see the actual compression results achieved


  Scenario: Debug capture records compaction events
    Given I am in an interactive session with debug capture enabled
    And I have conversation history
    When I type /compact
    Then a compaction.manual.start event should be recorded
    And a compaction.manual.complete or compaction.manual.failed event should be recorded
    And the events should contain token counts and compression metrics

