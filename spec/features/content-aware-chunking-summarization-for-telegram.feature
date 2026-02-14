@bridge
@chunking
@telegram
@BRIDGE-006
@critical @component @feature-group
Feature: Content-Aware Chunking Summarization for Telegram

  """
  Content type handlers for different streaming content: ThinkingHandler (summarizes thinking blocks), ToolCallHandler (formats tool invocations), ToolResultHandler (summarizes tool output), TextHandler (chunks prose). Each handler understands its content semantics.
  """

  Background: User Story
    As a developer monitoring AI sessions via Telegram
    I want to see concise summaries of thinking blocks and tool output
    So that I get essential information without verbose raw content flooding my chat


  Scenario: Thinking block shows condensed indicator
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Long thinking summarized with topic hint
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Tool call displays formatted invocation
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: File read tool result shows summary with line count
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Large tool output summarized not sent verbatim
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Tool call with arguments shows arg summary
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Multiple consecutive thinking blocks consolidated
    Given [precondition]
    When [action]
    Then [expected outcome]

