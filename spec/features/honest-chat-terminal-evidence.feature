@done
@BUG-263
@providers
@provider-abstraction
Feature: Honest Chat Terminal Evidence
  """
  Chat text and tool deltas remain progressive. Executable calls wait until
  finish reason plus consistent usage is closed by DONE or EOF. Malformed and
  error frames fail closed. Publish the validated physical final before releasing
  queued executable tools so the existing settlement hook runs before native Done
  can cancel consumption. JSON adaptation uses the same evidence/order contract.
  No executable calls are drained after accepted completion.
  """

  Background: User Story
    As a user relying on provider completion
    I want terminal errors and usage to be honest before Done is accepted
    So that completion cannot hide a failed or unbilled request

  Scenario: Usage without a finish reason cannot declare completion
    Given a Chat response with consistent usage but no finish reason
    When the streaming or JSON response ends
    Then the request fails once without successful completion or executable tools

  Scenario: Malformed and provider error frames retain actionable causes
    Given malformed Chat frames or a provider error followed by otherwise valid terminal evidence
    When the response is consumed
    Then the original failure is reported instead of silently declaring success

  Scenario: Done cannot conceal invalid terminal evidence
    Given a real armed Done call queued before invalid terminal evidence
    When the Chat response passes through normal attribution and the shared stream loop
    Then Done is not accepted and trailing work and ordinary continuation nudges do not execute

  Scenario: Valid native Done settles physical usage before immediate completion
    Given an attributed native Done call followed by trailing work with valid terminal evidence
    When streaming and JSON responses pass through the shared stream loop
    Then exactly one request is billed and its context is settled before accepted Done cancels trailing work

  Scenario: Explicit terminal marker settles without waiting for EOF
    Given a Chat connection sends finish reason and trailing usage followed by DONE but remains open
    When the response is consumed with a bounded deadline
    Then one final response contains exact usage and stop reason without requiring transport closure
