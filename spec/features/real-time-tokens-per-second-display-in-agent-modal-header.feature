@agent-modal
@tui
@TUI-031
Feature: Real-time tokens per second display in agent modal header

  """
  Modifies AgentModal.tsx component in src/tui/components/. Uses per-chunk delta calculation for accurate tok/s measurement. Tracks lastTokenUpdateRef for previous token count and timestamp. Collects instantaneous rate samples in tokPerSecSamples array. Displays average of all rate samples during streaming. Display position is between streaming indicator and token count in header Box component.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tokens per second is calculated from per-chunk deltas (deltaTokens / deltaTime between TokenUpdate events)
  #   2. Tokens per second display appears only during active streaming
  #   3. Tokens per second is displayed to the left of the token count in the header
  #   4. Display requires at least two TokenUpdate events to calculate a rate sample
  #   5. Value is formatted as 'X.X tok/s' with one decimal place
  #   6. Displayed value is the average of all instantaneous rate samples collected during streaming
  #
  # EXAMPLES:
  #   1. User sends a prompt, after multiple token updates, header shows averaged tok/s to the left of token count
  #   2. User sees nothing in the tok/s position until at least two TokenUpdate events have been received
  #   3. When streaming ends, the tok/s display disappears and only token counts remain visible
  #   4. Header shows: 'Agent: claude (streaming...)   12.3 tok/s   tokens: 1234↓ 567↑   [Tab] Switch'
  #   5. With slow provider, instantaneous rates are averaged for stable display
  #
  # ========================================

  Background: User Story
    As a developer using fspec agent modal
    I want to see real-time tokens per second during AI streaming
    So that I can monitor AI response performance and compare different provider speeds

  Scenario: Display tokens per second after multiple token updates
    Given the agent modal is open and streaming a response
    And multiple TokenUpdate events have been received
    And rate samples have been calculated from token deltas
    When the header is rendered
    Then the header should display the averaged tok/s value
    And the tokens per second should appear to the left of the token count


  Scenario: Suppress tokens per second before rate samples available
    Given the agent modal is open and streaming a response
    And only one TokenUpdate event has been received
    When the header is rendered
    Then no tokens per second value should be displayed


  Scenario: Hide tokens per second when streaming ends
    Given the agent modal was streaming a response
    And the tokens per second was being displayed
    When the streaming completes
    Then the tokens per second display should disappear
    And only the token counts should remain visible


  Scenario: Display proper header layout during streaming
    Given the agent modal is open with provider 'claude'
    And streaming is active with 12.3 tokens per second
    And 1234 input tokens and 567 output tokens have been used
    When the header is rendered
    Then the header should show 'Agent: claude' on the left
    And the header should show '(streaming...)' next to the provider name
    And the header should show '12.3 tok/s' to the left of the token count
    And the header should show 'tokens: 1234↓ 567↑'


  Scenario: Calculate tokens per second for slow provider
    Given the agent modal is open and streaming a response
    And multiple TokenUpdate events have been received with slow token generation
    When the header is rendered
    Then the header should display a low tok/s value reflecting the slow rate


  Scenario: Update tokens per second in real-time during streaming
    Given the agent modal is streaming a response
    And rate samples are being collected
    When additional tokens continue to stream over time
    Then the tokens per second display should update with new samples
    And the displayed value should reflect the average of all rate samples

