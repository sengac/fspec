@done
@BUG-268
@providers
@provider-abstraction
Feature: Copilot nonstreaming terminal evidence
  """
  Reuse the shared Responses terminal validator from both nonstreaming front
  doors. Never authorize tools on failure, truncation or safety termination.
  These are unchanged scenarios moved for native one-test-file traceability.
  """

  Background: User Story
    As a Copilot user
    I want nonstreaming completion to retain honest terminal semantics
    So that failed or unsafe output cannot authorize tool execution

  Scenario: Nonstreaming Responses failures cannot authorize tools or completion
    Given a local Copilot Responses endpoint returns failed or unsettled output containing a tool call
    When both public nonstreaming front doors consume that response
    Then each returns an error instead of a successful completion or tool request

  Scenario: Nonstreaming Responses truncation preserves its terminal semantics
    Given a local Copilot Responses endpoint returns token limited text and an unfinished tool call
    When the actual client model and provider consume that response
    Then text remains available with a maximum token stop and no executable tool call

  Scenario: Nonstreaming Responses safety stops cannot authorize tools
    Given a local Copilot Responses endpoint returns content filtered text alongside an unfinished tool call
    When the actual client model and provider consume that response
    Then the client retains the safety stop without executable tools and the provider cannot report ordinary completion
