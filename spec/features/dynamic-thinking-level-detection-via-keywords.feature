@thinking-detection
@tui
@napi
@high
@TOOL-010
Feature: Dynamic Thinking Level Detection via Keywords

  """
  Keyword detection happens in TypeScript (src/utils/thinkingLevel.ts) before sending to Rust session. Uses regex patterns to match command-like phrases while ignoring conversational usage. Disable keywords (quickly, briefly) have highest priority. Integrates with TOOL-009 ThinkingConfigFacade via getThinkingConfig() NAPI binding. Session receives thinking config JSON and merges into additional_params for provider request. UI displays detected level in status area. StreamChunk::Thinking type enables streaming thinking content to UI.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Disable keywords (quickly, briefly, fast, nothink) MUST have highest priority and always force ThinkingLevel::Off
  #   2. Thinking keywords MUST only match command-like phrases, NOT conversational usage (e.g., 'I think' should NOT trigger thinking)
  #   3. High-level keywords (ultrathink, think harder, think very hard) MUST map to ThinkingLevel::High
  #   4. Medium-level keywords (megathink, think hard, think deeply) MUST map to ThinkingLevel::Medium
  #   5. Low-level keywords (think about, think through, think carefully) MUST map to ThinkingLevel::Low
  #   6. Detected thinking level MUST be passed to getThinkingConfig() to generate provider-specific configuration
  #   7. The UI SHOULD display the detected thinking level as a visual indicator in the status area
  #
  # EXAMPLES:
  #   1. Prompt 'ultrathink about this bug' returns ThinkingLevel::High
  #   2. Prompt 'megathink through this problem' returns ThinkingLevel::Medium
  #   3. Prompt 'think about why this fails' returns ThinkingLevel::Low
  #   4. Prompt 'I think we should fix this' returns ThinkingLevel::Off (conversational, not a command)
  #   5. Prompt 'ultrathink but answer quickly' returns ThinkingLevel::Off (disable keyword wins)
  #   6. Prompt 'fix this bug' with no keywords returns ThinkingLevel::Off
  #   7. For Gemini 3 with ThinkingLevel::High, config includes thinkingLevel: 'high' (uses TOOL-009 facade)
  #   8. UI shows thinking indicator when level is not Off
  #
  # ========================================

  Background: User Story
    As a developer using fspec's AI agent
    I want to have thinking/reasoning levels automatically detected from my prompt keywords
    So that I can control how deeply the AI thinks without manual configuration, using natural language

  Scenario: High-level keyword ultrathink triggers maximum thinking
    Given I am composing a prompt in the agent modal
    When I type "ultrathink about this bug"
    Then the thinking level should be detected as High
    And the UI should display a thinking indicator showing "High"

  Scenario: Medium-level keyword megathink triggers moderate thinking
    Given I am composing a prompt in the agent modal
    When I type "megathink through this problem"
    Then the thinking level should be detected as Medium
    And the UI should display a thinking indicator showing "Medium"

  Scenario: Low-level keyword phrase triggers basic thinking
    Given I am composing a prompt in the agent modal
    When I type "think about why this fails"
    Then the thinking level should be detected as Low
    And the UI should display a thinking indicator showing "Low"

  Scenario: Conversational usage does not trigger thinking
    Given I am composing a prompt in the agent modal
    When I type "I think we should fix this"
    Then the thinking level should be detected as Off
    And the UI should NOT display a thinking indicator

  Scenario: Disable keyword overrides thinking keywords
    Given I am composing a prompt in the agent modal
    When I type "ultrathink but answer quickly"
    Then the thinking level should be detected as Off
    And the UI should NOT display a thinking indicator

  Scenario: Prompt without keywords defaults to no thinking
    Given I am composing a prompt in the agent modal
    When I type "fix this bug"
    Then the thinking level should be detected as Off
    And the UI should NOT display a thinking indicator

  Scenario: Detected level generates provider-specific config
    Given I am using the Gemini 3 provider
    And I have typed a prompt with "ultrathink"
    When the prompt is submitted
    Then getThinkingConfig should be called with provider "gemini-3" and level High
    And the thinking config should contain thinkingLevel set to "high"

  Scenario: Session receives thinking config on prompt submission
    Given I am composing a prompt with "megathink"
    And the thinking level has been detected as Medium
    When I submit the prompt
    Then the session prompt method should receive the thinking config
    And the config should be merged into additional_params for the provider

  Scenario: Thinking content is streamed with distinct chunk type
    Given I have submitted a prompt with thinking enabled
    When the provider returns thinking content
    Then the thinking content should be streamed as a Thinking chunk type
    And the UI should render the thinking content distinctly from regular text
