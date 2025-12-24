@done
@tui
@header
@context-window
@TUI-033
Feature: Context Window Fill Percentage Indicator

  """
  
  Implementation Architecture:
  - Backend (Rust): Add ContextFillUpdate event to stream types in codelet/core/src/stream/types.rs
  - Stream Loop: Calculate and emit context fill in codelet/cli/src/interactive/stream_loop.rs after token updates
  - NAPI Bridge: Expose ContextFillUpdate via codelet/napi/src/streaming.rs bindings
  - Frontend (React): Handle event in src/tui/components/AgentModal.tsx, add color-coded display
  
  Data Flow:
  - TokenTracker tracks cumulative tokens in session
  - Effective tokens = input_tokens - (cache_read_tokens * 0.9)
  - Threshold = context_window * 0.9 (compaction trigger)
  - Percentage = (effective_tokens / threshold) * 100
  
  Dependencies:
  - Existing TokenTracker infrastructure (codelet/core/src/compaction/model.rs)
  - Existing calculate_compaction_threshold() function (codelet/cli/src/compaction_threshold.rs)
  - NAPI streaming event system
  - Ink React components (Box, Text)
  
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Display shows just the percentage in brackets, e.g. [43%]
  #   2. Display is always visible, starting at [0%] before any tokens are used
  #   3. Color coding: green (0-49%), yellow (50-69%), magenta (70-84%), red (85%+)
  #   4. Percentage is calculated from effective tokens (cache-aware): effective = input_tokens - (cache_read_tokens * 0.9)
  #   5. Threshold for 100% is 90% of context window (compaction trigger point)
  #   6. Position: right of token count display, left of [Tab] Switch
  #
  # EXAMPLES:
  #   1. Fresh conversation: displays [0%] in green
  #   2. 45% fill: 81k effective tokens / 180k threshold = displays [45%] in green
  #   3. 60% fill: displays [60%] in yellow (warning approaching)
  #   4. 75% fill: displays [75%] in magenta (compaction warning)
  #   5. 90% fill: displays [90%] in red (compaction imminent)
  #   6. Cache effect: 150k raw input + 80k cached = 78k effective = [43%] not [83%]
  #   7. After compaction: percentage resets based on new effective token count
  #
  # ========================================

  Background: User Story
    As a developer using Claude Code
    I want to see how full the context window is at a glance
    So that I know when compaction is approaching and can plan accordingly

  Scenario: Display shows 0% at start of fresh conversation
    Given I start a fresh conversation in Claude Code
    And no tokens have been used yet
    When the AgentModal header renders
    Then I should see "[0%]" displayed in the header
    And the percentage should be colored green

  Scenario: Display shows percentage in green zone (0-49%)
    Given I am in a conversation with 81000 effective tokens used
    And the context window threshold is 180000 tokens
    When the AgentModal header renders
    Then I should see "[45%]" displayed in the header
    And the percentage should be colored green

  Scenario: Display shows percentage in yellow zone (50-69%)
    Given I am in a conversation with 108000 effective tokens used
    And the context window threshold is 180000 tokens
    When the AgentModal header renders
    Then I should see "[60%]" displayed in the header
    And the percentage should be colored yellow

  Scenario: Display shows percentage in magenta zone (70-84%)
    Given I am in a conversation with 135000 effective tokens used
    And the context window threshold is 180000 tokens
    When the AgentModal header renders
    Then I should see "[75%]" displayed in the header
    And the percentage should be colored magenta

  Scenario: Display shows percentage in red zone (85%+)
    Given I am in a conversation with 162000 effective tokens used
    And the context window threshold is 180000 tokens
    When the AgentModal header renders
    Then I should see "[90%]" displayed in the header
    And the percentage should be colored red

  Scenario: Percentage calculation uses effective tokens with cache discount
    Given I am in a conversation with 150000 raw input tokens
    And 80000 tokens are cache read tokens
    And the context window threshold is 180000 tokens
    When the effective token count is calculated
    Then the effective tokens should be 78000
    And I should see "[43%]" displayed in the header
    And the percentage should be colored green

  Scenario: Percentage resets after compaction
    Given I am in a conversation that has just been compacted
    And the new effective token count is 50000
    And the context window threshold is 180000 tokens
    When the AgentModal header renders after compaction
    Then I should see "[28%]" displayed in the header
    And the percentage should be colored green

  Scenario: Percentage indicator is positioned correctly in header
    Given I am in an active conversation
    When the AgentModal header renders
    Then the percentage indicator should appear after the token count display
    And the percentage indicator should appear before the Tab Switch component
