@done
@BUG-303
@cli
@streaming
@provider-abstraction
Feature: Malformed tool-call arguments are explained to the model instead of ending the turn
  """
  The rig multi-turn loop (streaming and non-streaming) normalizes every tool call before
  it is yielded, recorded or executed. Arguments that are not a JSON object (null, a
  non-object value, or a string that does not parse to an object) are replaced by an empty
  object in the recorded call, the tool is not executed, and the tool result tells the
  model what was received and how to call the tool using its own parameter schema.
  A string that parses to a JSON object is accepted as that object.

  The OpenAI chat request conversion also serializes any legacy non-object arguments as a
  JSON object so histories persisted before this fix replay without HTTP 400.

  Fix sites: patches/rig-core/src/tool/arguments.rs (normalization and guidance),
  patches/rig-core/src/agent/prompt_request/streaming.rs and mod.rs (loop integration),
  patches/rig-core/src/providers/openai/completion/mod.rs (wire conversion).
  """

  Background: User Story
    As a developer running agent sessions on OpenAI-compatible and other providers
    I want to have malformed tool calls explained back to the model instead of ending the turn with an API error
    So that the agent corrects its own tool usage and keeps working

  Scenario: A tool call with null arguments is not run and the model is told how to call the tool
    Given a model that calls the inspect tool with null arguments and then answers
    When the turn runs to completion
    Then the inspect tool is not executed
    And the tool result explains that inspect was not run and lists its required parameters
    And the follow-up request carries the tool call with empty JSON object arguments
    And the turn finishes with the model's answer and no error

  Scenario: A tool call with truncated JSON arguments reports what was received
    Given a model that calls the inspect tool with arguments truncated mid-object and then answers
    When the turn runs to completion
    Then the inspect tool is not executed
    And the tool result reports the received character count and an excerpt and suggests splitting large content

  Scenario: Stringified JSON object arguments are executed normally
    Given a model that calls the inspect tool with a JSON object encoded as a string and then answers
    When the turn runs to completion
    Then the inspect tool is executed once

  Scenario: Legacy history with null tool-call arguments replays as a valid request
    Given a session history containing a tool call whose arguments are null
    When the next request is sent to the OpenAI-compatible server
    Then the replayed tool call arguments are an empty JSON object
