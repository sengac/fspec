@done
@phase-1
@provider-abstraction
@providers
@BUG-219
@critical
Feature: Codex interruption status
  """
  RecoveryGuard owns the lifetime of each Codex stream. Its session-owned recovery handle forwards lifecycle notifications through the existing provider observer to session output. Cancellation is a recovery notification only while WaitingForNetwork or RetryingStream is active. Ordinary interruption must still drop the stream and release its resources.
  """

  Background: User Story
    As a Codex session owner
    I want interruption notifications to describe actual recovery
    So that ordinary cancellation does not imply a network outage

  Scenario: Ordinary setup and body cancellation remain silent
    Given a healthy Codex request awaiting initial setup or body data
    When the caller drops the pending stream before recovery begins
    Then the connection closes without a recovery status or notification

  Scenario: Active recovery cancellation is reported exactly once
    Given a Codex request waiting offline or retrying an interrupted stream
    When the caller drops the stream during active recovery
    Then exactly one cancellation notification ends recovery without a later request

  Scenario: Terminal completion never becomes cancellation
    Given a healthy or recovering Codex response completing with text or tools
    When the caller drops the stream immediately after final or tool completion
    Then completion keeps its terminal recovery status without cancellation
