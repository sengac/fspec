@done
@header
@watcher
@tui
@WATCH-015
Feature: Watcher Session Header Indicator

  """
  Architecture notes:
  - SplitSessionView receives additional props: displayModelId, displayReasoning, displayHasVision, displayContextWindow, tokenUsage, rustTokens, contextFillPercentage
  - Header layout: [watcher indicator] [capability badges] [context window] | right-side: [tokens] [context fill %]
  - Uses existing getContextFillColor() utility for context percentage color coding
  - formatContextWindow() utility formats large numbers (200000 → "200k")
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Watcher session header shows '👁️ Role Name (watching: Parent Name)' at the start
  #   2. Header includes model capability indicators [R] for reasoning, [V] for vision after the watcher indicator
  #   3. Header includes context window size in brackets (e.g., [200k])
  #   4. Header shows token usage on the right side (e.g., 'tokens: 1234↓ 567↑')
  #   5. Header shows context fill percentage with color coding (e.g., [45%])
  #   6. Watcher indicator text uses magenta/purple color to match watcher message styling
  #
  # EXAMPLES:
  #   1. User views Security Reviewer watcher watching Main Dev Session with claude-sonnet-4-20250514 model → header shows '👁️ Security Reviewer (watching: Main Dev Session) [R] [200k] tokens: 1234↓ 567↑ [45%]'
  #   2. Watcher using model with reasoning support → [R] indicator appears in magenta after the watcher info
  #   3. Watcher using model with vision support → [V] indicator appears in blue after capability indicators
  #   4. Context fill reaches 80% → percentage displays in yellow warning color
  #   5. Regular (non-watcher) session → header shows 'Agent: model-name' without watcher indicator (existing behavior unchanged)
  #   6. Turn select mode enabled in watcher view → [SELECT] indicator appears in header
  #
  # ========================================

  Background: User Story
    As a user viewing a watcher session
    I want to see the full header with watcher indicator, model info, and token stats
    So that I have all the same information as in a regular session plus the watcher context

  @critical
  Scenario: Full header displays for watcher session with all indicators
    Given a parent session "Main Dev Session" exists
    And a watcher session "Security Reviewer" is watching "Main Dev Session"
    And the watcher uses a model with reasoning support
    And the model has a context window of 200000 tokens
    And current token usage is 1234 input and 567 output
    And context fill is at 45 percent
    When I view the watcher session
    Then the header shows watcher indicator "👁️ Security Reviewer (watching: Main Dev Session)"
    And the header shows reasoning indicator "[R]" in magenta color
    And the header shows context window "[200k]"
    And the header shows token usage "tokens: 1234↓ 567↑"
    And the header shows context fill "[45%]"

  Scenario: Reasoning indicator appears for models with extended thinking
    Given a watcher session with reasoning-enabled model
    When I view the watcher session
    Then the header shows "[R]" indicator in magenta color
    And the indicator appears after the watcher info

  Scenario: Vision indicator appears for models with vision support
    Given a watcher session with vision-enabled model
    When I view the watcher session
    Then the header shows "[V]" indicator in blue color
    And the indicator appears after the reasoning indicator if present

  Scenario: Context fill percentage shows warning color at 80 percent
    Given a watcher session exists
    And context fill is at 80 percent
    When I view the watcher session
    Then the header shows "[80%]" in yellow warning color

  Scenario: Regular session header unchanged
    Given a regular session "Dev Session" exists
    And the session is not a watcher
    When I view the regular session
    Then the header shows "Agent:" followed by model name
    And the header does not show watcher indicator

  Scenario: SELECT indicator appears in watcher header during turn selection
    Given a watcher session exists
    And I am in turn select mode
    When I view the watcher session
    Then the header shows "[SELECT]" indicator in cyan color
