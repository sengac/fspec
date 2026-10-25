@done
@authentication
@BUG-269
@providers
@oauth
Feature: Scripted OAuth request refresh
  """
  Use the selected compiled engine AST and config at the existing shared
  build_headers boundary across all four completion front doors. Reuse the
  existing scripted token helper and preferred/legacy hooks, without another
  HTTP adapter or inference retry. Explicit Custom auth remains headers-owned.
  Credential-path locks contain no tokens; admit refresh under a root fence.
  An admitted synchronous refresh may persist after caller cancellation, but
  cannot send inference. See BUG-269/request-refresh-design.md and native review.
  """

  Background: User Story
    As a scripted OAuth user
    I want expired credentials refreshed before actual completions
    So that long-running requests use fresh authorization without overriding script ownership

  Scenario: Every real completion front door refreshes before authorization
    Given a selected scripted OAuth provider has expired synthetic tokens and a numeric loopback issuer
    When each streaming and nonstreaming front door completes with preferred or legacy refresh hooks
    Then the local issuer receives the old refresh token once and inference receives only fresh authorization
    And the new tokens are persisted with the ordinary completion and usage intact

  Scenario: Independent concurrent providers coalesce one credential refresh
    Given independently constructed scripted providers share one expired temporary credential store
    When their real completion requests run concurrently
    Then one issuer exchange precedes all freshly authorized inference requests

  Scenario: Existing script authentication ownership remains unchanged
    Given fresh missing tokens missing hooks or headers owned Custom authentication
    When actual custom completion requests are sent
    Then automatic refresh does not override existing ownership or duplicate a headers owned exchange

  Scenario: Project selected scripts remain authoritative during refresh
    Given project and global provider scripts disagree about refresh behavior
    When a normally selected project provider issues a real completion request
    Then its selected compiled script and OAuth config alone determine issuer headers and inference

  Scenario: Failed refreshes retain credentials and redact diagnostics
    Given a local issuer rejects refresh or fails transiently for expired synthetic credentials
    When an actual completion request reaches refresh
    Then no inference is sent no token changes and no credential or issuer body appears in diagnostics
    And a later explicit request can refresh successfully without automatic inference replay

  Scenario: Caller cancellation and deadline prevent inference after admitted refresh
    Given a synchronous local refresh has been observed at the issuer
    When the actual requesting task is cancelled or its caller deadline expires
    Then its authorized refresh may settle and persist but sends no inference
    And all admitted operations settle before the isolated fixture environment is restored

  Scenario: Credential root changes fail closed across refresh waits
    Given an admitted refresh and another waiting request share a temporary credential path
    When the active credential root changes before refresh persistence or waiter admission
    Then neither operation reads or writes the replacement credentials or sends inference
