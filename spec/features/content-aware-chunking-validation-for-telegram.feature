@bridge
@chunking
@telegram
@BRIDGE-006
@critical @component @feature-group
Feature: Content-Aware Chunking Validation for Telegram

  """
  Validates markdown completeness before sending - ensures no unclosed backticks, bold/italic markers, or broken code fences. Enforces Telegram's 4096 character limit by finding nearest logical boundary before limit and splitting there.
  """

  Background: User Story
    As a developer monitoring AI sessions via Telegram
    I want to receive messages with valid complete markdown
    So that formatting renders correctly and messages stay within platform limits


  Scenario: Message respects 4096 character limit
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Long message splits at logical boundary before limit
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Unclosed code block closed before sending
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Unclosed bold markers balanced before sending
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Each chunk has valid complete markdown
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Inline code backticks balanced in each chunk
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Code block exceeding limit truncated with indicator
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Truncated code block has closing fence
    Given [precondition]
    When [action]
    Then [expected outcome]

