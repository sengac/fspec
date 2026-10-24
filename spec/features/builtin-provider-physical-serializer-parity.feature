@done
@CMPCT-071
@providers
@context-management
@compaction
@phase-1
Feature: Builtin provider physical serializer parity
  """
  Admission prepares the selected streaming transport body using the same
  serializer as physical sends. Provider-specific instructions, retained
  messages, tool schemas, parameters, caching and streaming options cannot
  disappear from the admission certificate. Preparation performs no network
  request and does not mutate the source conversation. HTTP middleware body
  transformations participate in preparation, including Copilot cache markers.
  """

  Background: User Story
    As an agent promoting validated hierarchical memory
    I want admission bytes to equal the next selected provider request
    So that provider-specific overhead cannot bypass the capacity veto

  Scenario: OpenAI chat preparation matches streaming and nonstreaming sends
    Given selected OpenAI chat models with both transport modes and strict tools
    When requests containing mandatory context and parameters are prepared then sent
    Then prepared bytes equal the physical bodies including the selected stream options
    And preparation sends nothing and leaves the input request unchanged

  Scenario: Gemini retains documents and explicit generation limits without optional configuration
    Given a selected Gemini model with mandatory documents and explicit output limits
    When its request has no optional generation configuration and is prepared then sent
    Then prepared bytes equal the physical body including documents and the explicit output limits
    And preparation sends nothing and leaves the input request unchanged

  Scenario: OpenAI Responses preparation matches the streaming send
    Given a selected OpenAI Responses model with mandatory context and tool schemas
    When its request is prepared then sent
    Then prepared bytes equal the physical body including streaming and tool schemas
    And preparation sends nothing and leaves the input request unchanged

  Scenario: Anthropic preparation matches API key and OAuth streaming sends
    Given selected Anthropic API key and OAuth models with caching and tool schemas
    When requests containing mandatory context and parameters are prepared then sent
    Then prepared bytes equal the physical bodies including stable OAuth metadata
    And preparation sends nothing and leaves the input request unchanged

  Scenario: Gemini preparation matches the streaming send
    Given a selected Gemini model with mandatory context and generation parameters
    When its request is prepared then sent
    Then prepared bytes equal the physical body including generation parameters
    And preparation sends nothing and leaves the input request unchanged

  Scenario: Copilot preparation includes middleware transformations
    Given selected Copilot models using the actual HTTP middleware
    When requests containing mandatory context and parameters are prepared then sent
    Then prepared bytes equal the physical bodies including eligible cache markers
    And preparation sends nothing and leaves the input request unchanged
