@security
@session
@done
@BUG-404
@tools
@rust
Feature: Tool-layer isolation fails closed on failed session lookups
  """
  Worktree isolation was enforced only when
  codelet_tools::facade::get_isolation_context returned Some, and it returned
  None (no restriction) when session_tool_callbacks could not parse the id or
  the registered manager did not know the session. GetEffectiveCwdCallback
  now returns Result<Option<IsolationContext>, String>: Ok(Some) for an
  isolated session, Ok(None) for a found non-isolated session, Err for a
  malformed id, an unknown session or a missing manager.
  validate_and_resolve_path refuses the file operation with a validation error
  on Err, and get_effective_cwd returns the error to its callers. A process
  with no callback registered at all still means no isolation (tracked as a
  separate card, BUG-447); Uuid::nil stays the explicit no-session id, and an
  auxiliary agent registered with no owning session (tool_admission
  is_standalone_auxiliary, e.g. compaction outside a tool invocation) has no
  session or worktree behind it and stays unrestricted.
  Tests: rust/sessions/tests/bug404_tool_isolation_fails_closed.rs.
  """

  Background: User Story
    As a user running an isolated session
    I want a file operation for a session the tool layer cannot find to be refused
    So that a lookup failure never lifts the worktree boundary

  Scenario: A file operation for an unknown session is refused
    Given a session manager registered for the tool callbacks
    When a file path is validated for a session id the manager does not know
    Then the operation is refused with an error naming the session

  Scenario: A known non-isolated session is not restricted
    Given a session manager registered for the tool callbacks with a non-isolated session
    When a file path outside any worktree is validated for that session
    Then the path is allowed

  Scenario: A standalone auxiliary agent is not restricted
    Given a session manager registered for the tool callbacks
    And an auxiliary agent registered with no owning session
    When a file path is validated for that auxiliary agent
    Then the path is allowed
