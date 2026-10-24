@done
@BUG-241
@providers
@streaming
@integration
@provider-abstraction
Feature: Codex buffering wire compatibility
  """
  Match openai/codex bb64b894 codex-api/src/sse/responses.rs. Valid objects
  require string arrays use_cases and reasons. Present top-level values suppress
  nested fallback, including null and invalid values. Only response.metadata can
  supply nested buffering, moderation and verification metadata. Header advice
  is response scoped; an omitted retry_model uses that header and null clears it.
  HTTP treatment comes from physical response headers. WebSocket treatment comes
  from top-level event headers, not upgrade headers or nested response headers.
  Both HTTP and WebSocket streams use the same parser. Never expose private
  payloads or modify model selection. Supersedes PROV-153 boolean clearing policy.
  """

  Scenario: Ignore invalid buffering payloads
    Given Codex streams containing boolean null and malformed buffering payloads
    When those payloads arrive between valid buffering observations
    Then only valid objects affect buffering status
    And normal text and usage are delivered unchanged

  Scenario: Respect buffering source precedence
    Given conflicting top level and nested Codex buffering metadata
    When buffering metadata arrives on different event types
    Then top level presence wins and nested fallback requires a metadata event
    And private buffering reasons never enter diagnostics

  Scenario: Resolve faster model advice per event
    Given header advice and successive top level and nested buffering objects
    When retry model values are omitted null or explicit
    Then each event resolves its own advice without stale suggestions
    And the selected model remains unchanged

  Scenario: Scope optional metadata to metadata events
    Given moderation and verification metadata on recognized and unrelated events
    When Codex consumes those events over HTTP and WebSocket
    Then only recognized metadata events update optional status
    And private metadata is not exposed
