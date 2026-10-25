@done
@BUG-264
@providers
@provider-abstraction
Feature: Responses Terminal Semantics
  """
  Ordinary Responses is distinct from dedicated Codex. Separate the oversized
  stream schema, request and decoder mechanically before terminal repair.
  Preserve failed backend causes, normalize incomplete reasons and validate
  terminal status and usage before releasing calls. Publish physical final before
  executable calls for existing settlement hooks. Keep native call_id identity.
  Dedicated Codex keeps its existing terminal policy and receives only the shared
  final response field compatibility default.
  """

  Background: User Story
    As a user of an ordinary Responses adapter
    I want failures and incomplete outcomes to remain distinguishable
    So that completion and recovery do not discard the provider outcome

  Scenario: Failed Responses retain backend causes without tools
    Given a failed Responses terminal with a backend code and message and queued tools
    When its real local stream is consumed
    Then the typed failure cause survives without successful final or executable calls

  Scenario: Incomplete Responses preserve normalized recovery metadata
    Given max output or content filtered incomplete terminals with consistent usage and queued tools
    When their real local streams are consumed
    Then the final metadata preserves the incomplete reason and exact usage without executable calls

  Scenario: Invalid terminal evidence cannot be hidden by native Done
    Given an attributed armed Done call queued before missing or inconsistent terminal usage
    When the response passes through the real shared stream loop
    Then Done is not accepted and no trailing effect or ordinary nudge occurs

  Scenario: Completed native Done settles once with paired invocation identities
    Given a valid Responses terminal containing native Done and trailing work
    When the response passes through normal admission and the shared stream loop
    Then physical usage is billed once before accepted Done and its call ID remains paired while trailing work is cancelled

  Scenario: Explicit backend errors and metadata completions retain their contracts
    Given an explicit backend error or a valid metadata only completed terminal
    When the real local Responses request is consumed
    Then the error is actionable or the completion contains exact usage without inventing tools
