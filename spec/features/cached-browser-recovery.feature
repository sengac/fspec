@done
@phase-1
@browser
@web-tools
@error-handling
@BUG-237
Feature: Cached browser recovery
  """
  WebSearch shares a cached CDP browser. Probe cached transports before reuse,
  replace only the failed generation under one cache lock, and retry once.
  Recovery retains the original Chrome configuration and never takes ownership
  of an externally launched browser. Installed-Chrome expiry tests are separate
  from deterministic cache-policy tests.
  """

  Scenario: Reuse a healthy browser
    Given a healthy cached browser
    When another operation requests the same browser mode
    Then the same browser is reused

  Scenario: Bound recovery when the replacement also fails
    Given a browser operation fails because its transport is closed
    When the replacement operation also fails
    Then the failure is returned after exactly one retry

  Scenario: Preserve healthy browsers on page failures
    Given a healthy cached browser
    When an operation fails with a page or script error
    Then the error is returned without replacing the browser

  Scenario: Concurrent stale callers share one replacement
    Given two operations hold the same failed cached browser
    When both operations request recovery
    Then both use one healthy replacement
    And a late stale failure does not evict that replacement

  Scenario: Preserve the requested browser mode during recovery
    Given a failed cached browser in visible mode
    When the operation recovers
    Then the replacement remains in visible mode
