@RPC-419
@done
@tui
@header
@context-window
@TUI-033
Feature: Context Window Fill Percentage Indicator
  """
  Implementation Architecture:
  - Backend (Rust): ContextFillUpdate event in stream types (rust/core/src/stream/types.rs)
  - Stream Loop: emit_context_fill_from_usage in rust/cli/src/interactive/stream_loop.rs calculates and emits context fill after token updates
  - NAPI Bridge: Expose ContextFillUpdate via rust/napi/src/streaming.rs bindings
  - Frontend (React): Handle event in src/tui/components/AgentView.tsx, add color-coded display

  Data Flow (corrected by RPC-419):
  - The backend's emit_context_fill_from_usage emits effective_tokens = ApiTokenUsage::total_context() = input + cache_read + cache_creation + output + reasoning tokens — physical context-window occupancy with NO cache discount
  - Percentage = trunc((total_context / threshold) * 100)
  - Threshold = context_window * 0.9 (compaction trigger), from calculate_compaction_threshold()
  - The 0.9 cache-read discount belongs EXCLUSIVELY to compaction's TokenTracker::effective_tokens (compaction scheduling heuristic in rust/core/src/compaction/model.rs); it plays no part in the fill percentage
  - The frontend displays the backend-supplied fill percentage verbatim

  Dependencies:
  - Existing TokenTracker infrastructure (rust/core/src/compaction/model.rs)
  - Existing calculate_compaction_threshold() function (rust/cli/src/compaction_threshold.rs)
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

  Scenario: Percentage displays the backend's physical-occupancy calculation verbatim
    Given the backend has computed a fill percentage of 43 from 78000 total context tokens (input + cache + output + reasoning, with no cache discount) against a threshold of 180000 tokens
    When the backend emits ContextFillUpdate with fill_percentage=43
    Then the frontend displays the backend-supplied "[43%]" verbatim in the header
    And the percentage should be colored green
