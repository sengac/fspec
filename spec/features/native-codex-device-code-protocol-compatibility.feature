@done
@authentication
@oauth
@BUG-279
@providers
@integration
@validation
@critical
Feature: Native Codex device-code protocol compatibility
  """
  Native TUI Headless action uses embedded RPC and the existing device_session LoginAttempt owner. Align JSON start/poll and callback exchange with pinned official Codex 6ba3a84680236c621b93e0b8dc140687a37f41fc; retain shared timeout, cancellation, account-ID extraction and credential commit checks. HTTP403/404 are bounded pending, never authorization. Sanitize failure paths; no response body or codes in errors/logs. Numeric interval compatibility remains intentional. All fixtures use loopback HTTP, synthetic credentials and mock keyring; no real login or broader auth refactor.
  """

  Background: User Story
    As a Codex user
    I want to sign in through the native device-code flow
    So that I can authorize safely or receive a clear redacted error

  Scenario: Official usercode response formats are accepted
    Given official usercode aliases and string missing or numeric polling intervals
    When the native provider requests a device code using JSON
    Then the code and normalized interval are returned without weakening malformed input checks

  Scenario: Pending HTTP statuses do not require JSON bodies
    Given a native device login whose polling endpoint returns empty pending HTTP403 and HTTP404
    When authorization later succeeds with a valid polling response
    Then both original identifiers accompany each JSON poll and callback exchange persists valid tokens

  Scenario: Malformed and terminal responses cannot authorize exchange
    Given malformed usercode or poll responses and terminal HTTP or protocol failures
    When native device authorization processes those responses
    Then it fails explicitly without exchange persistence or secret disclosure

  Scenario: Timeout and cancellation preserve credential ownership
    Given a pending native device login and existing synthetic credentials
    When the operation times out or is cancelled during authorization
    Then no late completion replaces credentials and a fresh attempt can proceed

  Scenario: Failed exchange or missing account identity cannot replace credentials
    Given a successful device poll and existing synthetic credentials
    When token exchange fails or returns no usable account identity
    Then the native login fails and existing credential bytes remain unchanged
