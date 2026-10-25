@done
@BUG-261
@providers
@provider-abstraction
@authentication
Feature: Codex Forced Refresh Recovery
  """
  Forced and subsequent proactive refresh share typed NetworkFailure recovery.
  Preserve original refresh rejection errors; translate setup timeouts to the
  existing typed deadline. Keep credential synchronization and token-race guards.
  Local HTTP and isolated synthetic credential fixtures exercise the full model.
  """

  Background: User Story
    As a Codex OAuth user
    I want to recover from an issuer outage during forced credential refresh
    So that temporary network failures do not falsely demand signing in again

  Scenario: Forced refresh network failures enter cancellable recovery
    Given a real inference request rejected with unauthorized credentials
    When forced refresh encounters an offline issuer or setup deadline
    Then the original request enters typed recovery and cancellation stops it promptly

  Scenario: Credential rejection remains terminal after unauthorized inference
    Given a real inference request rejected with unauthorized credentials
    When the issuer rejects refresh or returns malformed credentials
    Then the actionable credential error terminates without network recovery or secrets

  Scenario: Repeated refresh deadlines retain bounded retry behavior
    Given unauthorized inference followed by an issuer that repeatedly exceeds setup deadlines
    When the original request exhausts its existing stream retry budget
    Then the typed network failure reports five retries without replaying inference

  Scenario: Forced refresh retains typed failure causes at the HTTP boundary
    Given a loopback inference request whose path cannot be rewritten externally
    When unauthorized inference forces refresh against an offline or stalled issuer
    Then the original typed connection or setup deadline cause survives

  Scenario: Approved test transport is restricted to numeric loopback HTTP
    Given the approved test only endpoint injection seam
    When local and nonlocal endpoint candidates are configured
    Then only numeric loopback HTTP is accepted and the shipping rewrite policy stays unchanged

  Scenario: Missing synthetic credential homes return controlled fixture errors
    Given an isolated synthetic credential home that has been removed
    When the reused fixture attempts to resolve its keychain
    Then it returns a filesystem error instead of panicking
