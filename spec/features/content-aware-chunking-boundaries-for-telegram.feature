@bridge
@chunking
@telegram
@BRIDGE-006
@critical @component @feature-group
Feature: Content-Aware Chunking Boundaries for Telegram

  """
  Implements a content-aware buffer that accumulates streaming data and flushes at detected boundaries. Uses boundary priority: code block > heading > paragraph > sentence > max size. Integrates with the telegram-endpoint.ts message processing pipeline.
  """

  Background: User Story
    As a developer monitoring AI sessions via Telegram
    I want to receive messages that break at natural content boundaries
    So that messages are readable without awkward mid-sentence or mid-block splits


  Scenario: Complete sentence arrives in single message
    When [action]
    Then [expected outcome]


  Scenario: Chunk ends at sentence boundary not mid-word
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Code block arrives as complete unit
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Multi-line code block never splits across messages
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Paragraph break triggers new chunk
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Heading starts new message
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: List items stay together in single message
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Boundary priority code block over paragraph
    Given [precondition]
    When [action]
    Then [expected outcome]


  Scenario: Boundary priority heading over sentence
    Given [precondition]
    When [action]
    Then [expected outcome]

