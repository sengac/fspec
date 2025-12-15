@session-management
@context-management
@agent
@high
@CLI-009
Feature: Context Compaction with Anchoring System

  """
  LLM-based summarization uses current ProviderManager with 3 retry attempts (exponential backoff: 0ms, 1000ms, 2000ms). Summary injected as user message with session continuation message. Prompt cache cleared after compaction. AUTOCOMPACT_BUFFER = 50k tokens.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Compaction triggers at 90% of context window using effective tokens (accounting for 90% prompt cache discount)
  #   2. Anchor points must have 90% or higher confidence to be detected (high precision over recall)
  #   3. Always preserve last 2-3 complete conversation turns (recent context protection)
  #   4. Compression ratio must be >= 60% or emit warning suggesting fresh conversation
  #   5. Error resolution anchors have weight 0.9, task completion anchors have weight 0.8
  #   6. LLM summary generation uses 3 retry attempts with exponential backoff (0ms, 1000ms, 2000ms)
  #   7. Prompt cache must be cleared after compaction since context has changed significantly
  #
  # EXAMPLES:
  #   1. User has 90 conversation turns totaling 85k tokens in 100k context window → compaction triggers → turns compacted to 30k tokens → 65% compression ratio
  #   2. Turn has previous error + Edit tool call + test pass result → detected as error-resolution anchor with 0.95 confidence and 0.9 weight
  #   3. Turn has no previous error + Write tool call + test success → detected as task-completion anchor with 0.92 confidence and 0.8 weight
  #   4. 90 turns total, anchor at turn 40 → keeps turns 40-90 (51 turns), summarizes turns 1-39 → 43% of turns summarized
  #   5. Compaction achieves only 45% compression ratio → emits warning 'Compression ratio below 60% - consider starting fresh conversation'
  #   6. LLM summary generation fails on first try → retries after 1s delay → succeeds → summary injected as user message
  #   7. After compaction, messages array contains: system messages + kept turns + summary message + session continuation message
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we use rig's built-in token usage tracking or implement our own estimation like codelet does?
  #   A: Use custom TokenTracker struct that preserves cache_read_input_tokens and cache_creation_input_tokens by accessing rig's anthropic::completion::Usage directly (before lossy conversion to generic crate::completion::Usage). See attached token-tracking-architecture.md for full technical analysis.
  #
  #   Q: What should AUTOCOMPACT_BUFFER be for Rust implementation (codelet uses 50k tokens)?
  #   A: Use 50,000 tokens (50k) matching codelet's AUTOCOMPACT_BUFFER constant. This reserves buffer space for compaction operations and summary generation.
  #
  #   Q: Should compaction emit user-visible notifications like '[Generating summary...]' and '[Context compacted]' during execution?
  #   A: Yes - emit '[Generating summary...]' when compaction starts and '[Context compacted]' with metrics when complete. Match codelet's user-visible notifications (lines 844, 930-942 in runner.ts) for transparency.
  #
  #   Q: Should the LLM summary provider use the current provider from ProviderManager or always use a specific provider?
  #   A: Use current provider from ProviderManager, matching codelet's design (llm-summary-provider.ts:88). This ensures summary generation uses the same model the user selected for the session.
  #
  #   Q: Should we store conversation turns in Session or create a new ConversationManager module?
  #   A: Store conversation turns in Session alongside messages vector. Session owns both: messages (for rig API) and turns (for anchor detection). No separate ConversationManager - keep it simple following Factory AI's 'simplicity over sophistication' principle.
  #
  # ========================================

  Background: User Story
    As a developer using codelet for long coding sessions
    I want to have intelligent context compaction with anchor point detection
    So that conversations can continue beyond context limits while preserving critical information

  Scenario: Trigger compaction at 90% context window using effective tokens
    Given I have a session with 100k token context window
    And the session has 90 conversation turns totaling 85k tokens
    And effective tokens account for 90% cache discount
    When I calculate if compaction should trigger
    Then compaction should trigger (85k > 90k threshold)
    And turns are compacted to 30k tokens
    And compression ratio is 65%

  Scenario: Detect error-resolution anchor with high confidence
    Given I have a conversation turn
    And the turn has previous_error flag set to true
    And the turn contains Edit tool call
    And the turn contains test pass result
    When I run anchor point detection
    Then an error-resolution anchor is detected
    And anchor confidence is 0.95
    And anchor weight is 0.9

  Scenario: Detect task-completion anchor with high confidence
    Given I have a conversation turn
    And the turn has previous_error flag set to false
    And the turn contains Write tool call
    And the turn contains test success result
    When I run anchor point detection
    Then a task-completion anchor is detected
    And anchor confidence is 0.92
    And anchor weight is 0.8

  Scenario: Select turns for compaction using anchor-based strategy
    Given I have 90 total conversation turns
    And an anchor point exists at turn 40
    When I select turns for compaction
    Then turns 40-89 are kept (50 turns, 0-indexed)
    And turns 0-39 are summarized (40 turns, 0-indexed)
    And compression estimate is 44.4% (40/90 turns summarized)

  Scenario: Emit warning for low compression ratio
    Given compaction has been executed
    And compression ratio is 45%
    When I validate compression quality
    Then a warning is emitted
    And warning message is "Compression ratio below 60% - consider starting fresh conversation"

  Scenario: Retry LLM summary generation with exponential backoff
    Given LLM summary generation fails on first attempt
    When retry logic executes
    Then retry attempt 1 waits 0ms
    And retry attempt 2 waits 1000ms
    And retry attempt 3 waits 2000ms
    And summary is generated successfully on retry
    And summary is injected as user message

  Scenario: Reconstruct messages after compaction
    Given compaction has completed successfully
    When messages are reconstructed
    Then messages array contains system messages
    And messages array contains kept turns
    And messages array contains summary message
    And messages array contains session continuation message
    And message order preserves append-only structure
