@done
@critical
@cli
@token-tracking
@bug-fix
@CTX-003
Feature: Fix Token Inflation Bug (15.7x Overcounting)
  """

  ROOT CAUSE: Two compounding bugs in token tracking:

  BUG 1 - SEMANTIC CONFUSION:
  - Anthropic API input_tokens = TOTAL context size per call (absolute)
  - Code treats it as INCREMENTAL tokens added (relative)
  - Example: 3 API calls report 50k, 55k, 60k (all absolute context sizes)
  - Buggy sum: 50k + 55k + 60k = 165k
  - Correct: just show 60k (current context)

  BUG 2 - DOUBLE COUNTING:
  - current_api_input added in FinalResponse: turn_accumulated += current_api_input
  - Same value added again in next MessageStart: turn_accumulated += current_api_input
  - Pattern: first call counted once, subsequent calls counted TWICE
  - 3 calls at 50k, 55k, 60k → actually accumulates 270k (not 165k)

  AFFECTED CODE PATHS:
  1. stream_loop.rs:401 - MessageStart adds previous API's tokens
  2. stream_loop.rs:448 - FinalResponse adds current API's tokens
  3. stream_loop.rs:811 - Session tracker accumulates turn total
  4. stream_loop.rs:417-420 - Display emits prev + accumulated + current

  FIX STRATEGY:
  - Track current_context_tokens (overwrite with latest input_tokens)
  - Track cumulative_billed_input separately (sum for billing analytics)
  - Display uses current_context_tokens only
  - Remove double-counting by not adding in MessageStart

  UNAFFECTED (WORKING CORRECTLY):
  - CompactionHook.on_stream_completion_response_finish OVERWRITES state.input_tokens
  - Compaction threshold checks use correct per-request values

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Anthropic API input_tokens represents the TOTAL context size for each API call, not incremental tokens added
  #   2. Display token count MUST show the current context size (latest input_tokens value), not cumulative sum of all API calls
  #   3. Session persistence MUST distinguish between current_context_tokens (latest value) and cumulative_billed_tokens (sum for billing)
  #   4. Compaction threshold checking MUST use per-request token values, not cumulative sums
  #   5. Multi-API-call turns (tool use) MUST NOT multiply token counts - each API call reports full context, not incremental
  #   6. Token display inflation ratio MUST be 1:1 (actual context to displayed) regardless of number of API calls
  #
  # EXAMPLES:
  #   1. Single API call: User sends message, API reports input_tokens=50,000 and output_tokens=2,000. Display shows 'Input: 50,000 tokens'. Session stores current_context=50,000.
  #   2. Multi-API turn with 3 tool calls: API Call 1 reports input=50,000, API Call 2 reports input=55,000, API Call 3 reports input=60,000. CORRECT display shows 'Input: 60,000 tokens' (latest). BUGGY display shows 'Input: 165,000 tokens' (sum).
  #   3. Real-world session with 15.7x inflation: Session has 142 messages, actual context=105,000 tokens, but session shows total_input_tokens=1,658,550. This means approximately 15.8 API calls occurred (1,658,550/105,000), each reporting full context that got summed.
  #   4. Turn with 10 tool calls: Context grows from 100k to 110k (10k added). Each of 10 API calls reports ~105k average input_tokens. BUGGY: accumulates 10*105k=1,050,000. CORRECT: shows final context 110,000.
  #   5. Compaction threshold check: threshold=180,000, actual context=105,000. CORRECT: no compaction triggered (105k < 180k). BUGGY with cumulative tracking: 1,658,550 > 180k would always trigger compaction.
  #   6. CompactionHook behavior (currently correct): on_stream_completion_response_finish OVERWRITES state.input_tokens with per-request value, so compaction checks use actual context size. This is why compaction still works despite the display bug.
  #   7. Session persistence after fix: session.json stores {current_context_tokens: 105000, cumulative_billed_input: 1658550, cumulative_billed_output: 9733}. Display shows current context (105k), billing analytics show cumulative (1.65M).
  #   8. Stream display during multi-call turn: As each API call completes, display updates to show CURRENT context size. User sees: 50k -> 55k -> 60k (correct progression). Not: 50k -> 105k -> 165k (cumulative bug).
  #   9. Billing vs Context distinction: After 5 API calls with context at 100k, billing total is 500k (5*100k for cost calculation), but context display shows 100k (for user understanding of window usage).
  #   10. Context fill percentage: With context_window=200k and threshold=180k, fill should show 58% when context is 105k (105/180). NOT 920% (1658550/180000) from cumulative bug.
  #
  # ========================================
  Background: User Story
    As a developer using codelet CLI
    I want to see accurate token counts that reflect my current context size
    So that I can trust the displayed metrics and make informed decisions about context management

  @unit-test
  Scenario: Display accurate tokens for single API call
    Given I am using the codelet CLI in an interactive session
    And the session has no previous token usage
    When I send a message that triggers a single API call
    And the API reports input_tokens of 50000 and output_tokens of 2000
    Then the token display should show "Input: 50,000 tokens"
    And the session should store current_context_tokens as 50000
    And the inflation ratio should be 1:1

  @unit-test
  Scenario: Display accurate tokens for multi-API turn with tool calls
    Given I am using the codelet CLI in an interactive session
    And the session has no previous token usage
    When I send a message that triggers 3 sequential API calls with tool use
    And API Call 1 reports input_tokens of 50000
    And API Call 2 reports input_tokens of 55000
    And API Call 3 reports input_tokens of 60000
    Then the final token display should show "Input: 60,000 tokens"
    And the display should NOT show "Input: 165,000 tokens"
    And the session should store current_context_tokens as 60000

  @unit-test
  Scenario: Prevent 15.7x token inflation in real-world session
    Given I am using the codelet CLI in an interactive session
    And the session has accumulated 142 messages
    And the actual current context is 105000 tokens
    When approximately 15-16 API calls have occurred during the session
    Then the token display should show approximately 105000 tokens
    And the display should NOT show 1658550 tokens
    And the inflation ratio should remain 1:1

  @unit-test
  Scenario: Accurate display during turn with 10 tool calls
    Given I am using the codelet CLI in an interactive session
    And the initial context is 100000 tokens
    When a turn triggers 10 API calls with tool use
    And each API call adds approximately 1000 tokens to the context
    And API calls report input_tokens of 100000, 101000, 102000, 103000, 104000, 105000, 106000, 107000, 108000, 109000
    Then the final token display should show 109000 tokens
    And the display should NOT show 1045000 tokens from cumulative summing
    And the session should store current_context_tokens as 109000

  @unit-test
  Scenario: Compaction threshold uses correct per-request tokens
    Given I am using the codelet CLI in an interactive session
    And the compaction threshold is set to 180000 tokens
    And the actual current context is 105000 tokens
    When the compaction threshold is checked
    Then compaction should NOT be triggered
    And the threshold check should use 105000 tokens not cumulative sums

  @unit-test
  Scenario: CompactionHook correctly overwrites with per-request value
    Given the CompactionHook is processing API responses
    When on_stream_completion_response_finish receives input_tokens of 105000
    Then state.input_tokens should be set to 105000 by overwriting
    And state.input_tokens should NOT be incremented by adding

  @unit-test
  Scenario: Session persistence stores both context and billing tokens
    Given I am using the codelet CLI in an interactive session
    And the current context is 105000 tokens
    And the cumulative billed input is 1658550 tokens
    When the session is persisted to session.json
    Then the session file should contain current_context_tokens of 105000
    And the session file should contain cumulative_billed_input of 1658550
    And the token display should use current_context_tokens for user display

  @unit-test
  Scenario: Stream display shows progressive context size not cumulative
    Given I am using the codelet CLI in an interactive session
    When a turn triggers multiple API calls
    And API Call 1 completes with input_tokens of 50000
    And API Call 2 completes with input_tokens of 55000
    And API Call 3 completes with input_tokens of 60000
    Then the display should progressively show "50k -> 55k -> 60k"
    And the display should NOT show "50k -> 105k -> 165k"

  @unit-test
  Scenario: Distinguish billing total from context display
    Given I am using the codelet CLI in an interactive session
    And 5 API calls have occurred with context at 100000 tokens each
    When viewing the token metrics
    Then the context display should show 100000 tokens for window usage
    And the billing analytics should show 500000 cumulative billed tokens
    And these should be clearly distinguished as separate metrics

  @unit-test
  Scenario: Context fill percentage uses current context not cumulative
    Given I am using the codelet CLI in an interactive session
    And the context_window is 200000 tokens
    And the compaction threshold is 180000 tokens
    And the current context is 105000 tokens
    When the context fill percentage is calculated
    Then the fill percentage should be approximately 58 percent
    And the fill percentage should NOT be 920 percent from cumulative tracking

  @unit-test
  Scenario: Prevent double-counting tokens between MessageStart and FinalResponse
    Given I am using the codelet CLI in an interactive session
    And a turn has multiple API calls
    When API Call 1 triggers MessageStart with input_tokens of 50000
    And API Call 1 triggers FinalResponse with input_tokens of 50000
    And API Call 2 triggers MessageStart with input_tokens of 55000
    And API Call 2 triggers FinalResponse with input_tokens of 55000
    Then the turn_accumulated_input should NOT include 50000 twice
    And the turn_accumulated_input should NOT include 55000 twice
    And the final accumulated value should be 55000 not 160000
    And each API call's tokens should only be counted once

  @unit-test
  Scenario: Verify inflation calculation with both bugs present
    Given the current buggy implementation is in place
    When 3 API calls occur with input_tokens of 50000, 55000, and 60000
    Then Bug 1 semantic confusion would sum to 165000 tokens
    And Bug 2 double-counting would inflate to 270000 tokens
    But the correct display should show only 60000 tokens
    And the ratio of buggy to correct is 4.5x for 3 API calls
