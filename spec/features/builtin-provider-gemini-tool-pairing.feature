@done
@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Builtin Gemini tool result pairing
  """
  Gemini correlates native function responses by function name, while persisted
  generic history correlates tool results by tool call identity. Shared request
  serialization must resolve each result against its preceding call without
  changing the source history, arguments, complete result or chronological order.
  Preparation, streaming and direct completion use the same conversion.
  """

  Background: User Story
    As an agent resuming structured tool history with Gemini
    I want native function responses to identify their original functions
    So that preserved tool evidence remains usable after compaction

  Scenario: Streaming Gemini pairs distinct tool identities with function names
    Given a selected Gemini model with distinct call identifiers and two named functions
    When the history is prepared for admission and physically streamed
    Then every native response names its original function with complete ordered evidence
    And prepared bytes match physical bytes without mutating source identities

  Scenario: Direct Gemini pairs distinct tool identities with function names
    Given a selected Gemini model with distinct call identifiers and two named functions
    When the history completes through the physical HTTP adapter
    Then every native response names its original function with complete ordered evidence
    And prepared bytes match physical bytes without mutating source identities
