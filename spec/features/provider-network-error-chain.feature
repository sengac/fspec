@phase-1
@cli
@providers
@provider-abstraction
@regression
Feature: Classify transport failures through provider error source chains
  """
  Architecture notes:
  The shared stream recovery decision inspects the complete error source chain.
  A body decoding wrapper is not itself evidence of a transport failure.
  Codex continues to own its retries even when an outer error looks transient.
  """

  Scenario: Wrapped transport timeouts remain retryable for ordinary providers
    Given a provider body decoding error wraps a transport timeout
    When the shared recovery policy classifies the error
    Then ordinary providers can retry the transport failure

  Scenario: Invalid response data is not mistaken for a transport failure
    Given a provider body decoding error wraps invalid response data
    When the shared recovery policy classifies the error
    Then ordinary providers do not retry the decoding failure

  Scenario: Provider-owned retry exhaustion overrides transient wrapper messages
    Given a Codex retry exhaustion error is wrapped in a transport timeout message
    When the shared recovery policy classifies the error
    Then no caller starts another network retry loop
