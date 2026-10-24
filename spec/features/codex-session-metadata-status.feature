@done
@BUG-241
@PROV-153
@providers
@streaming
@integration
@provider-abstraction
Feature: Codex session metadata status
  """
  The session owns a private Codex status handle. Sanitized changes use the existing
  diagnostic channel without announcing routine snapshots. BUG-241 separates
  transient waiting state from actionable verification and recovery notifications.
  Ordinary providers never bind the handle or emit Codex notifications.
  """

  Scenario: Session presentation retains diagnostics without routine notices
    Given a Codex session connected to a local response stream
    When header metadata and buffering updates arrive
    Then the session retains safe status without routine user notifications
    And a non-Codex session emits no Codex notifications
