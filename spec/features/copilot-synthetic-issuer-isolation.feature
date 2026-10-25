@done
@BUG-268
@providers
@testing
Feature: Copilot synthetic issuer isolation
  """
  Explicit in-process issuer overrides exist only under nondefault test-support.
  They never alter shipping endpoints or read personal credentials. Reject
  nonnumeric/nonloopback URLs, userinfo, fragments and redirect escapes before
  full expired-token/provider fixtures. Existing refresh policy is unchanged.
  """

  Background: User Story
    As a developer validating Copilot refresh without external services
    I want a narrowly isolated synthetic issuer seam
    So that acceptance tests cannot send credentials outside disposable loopback fixtures

  Scenario: Unsafe issuer destinations are rejected before requests
    Given an isolated temporary Copilot credential root
    When test overrides use DNS nonloopback userinfo or fragment destinations
    Then each override is rejected before any issuer or inference request

  Scenario: Redirecting synthetic issuers cannot escape loopback fixtures
    Given a validated local synthetic issuer redirects to another endpoint
    When the seam probes the issuer with a synthetic token
    Then the redirect is rejected without contacting its destination

  Scenario: A validated seam preserves isolated synthetic exchange behavior
    Given a local synthetic issuer and an isolated temporary credential root
    When the seam probes that issuer with a synthetic token
    Then only the local issuer receives that token and returns the synthetic exchange

  Scenario: Changed credential roots fail closed while a fixture is active
    Given an active synthetic issuer bound to a temporary credential root
    When the credential location changes directly or through a symlink
    Then exchange fails before contacting the issuer and cannot fall back externally

  Scenario: Synthetic issuer responses cannot select an external inference origin
    Given a local synthetic issuer returns an external inference origin
    When the isolated exchange response is validated
    Then the response is rejected without any inference request

  Scenario: Synthetic issuer traffic cannot escape through inherited proxies
    Given a validated local synthetic issuer and a different local proxy capture
    When proxy environment points at that proxy without a loopback exemption
    Then the issuer receives the synthetic exchange directly and the proxy receives nothing
