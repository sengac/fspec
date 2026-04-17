@PROV-051
Feature: Fireworks.ai Session Affinity for Prompt Cache Optimization
  """
  Session affinity header set via rig ClientBuilder::http_headers() at client construction time in OpenAIProvider::from_api_key_with_options()
  from_api_key_with_options() needs a session_id parameter (or reads OPENAI_SESSION_AFFINITY env var) to set the header value
  No rig-core changes needed — http_headers() and PromptTokensDetails already exist
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. When OPENAI_BASE_URL points to a Fireworks.ai endpoint, an x-session-affinity header MUST be sent with every request
  #   2. The session affinity value MUST be stable per codelet session (use the session UUID)
  #   3. The header MUST be set at client construction time via rig's http_headers() so it applies to all requests (both streaming and non-streaming)
  #   4. Non-Fireworks OpenAI-compatible endpoints (vLLM, Ollama, LM Studio, standard OpenAI) MUST NOT be affected — the header is harmless but should ideally only be sent when useful
  #   5. Cached token counts from Fireworks responses MUST be captured in usage metrics (already handled by rig-core's PromptTokensDetails.cached_tokens deserialization)
  #   6. The session affinity identifier can optionally be configured via OPENAI_SESSION_AFFINITY env var, defaulting to the codelet session UUID
  #   7. Send unconditionally for all custom base URL endpoints. The header is harmless — servers that don't understand it will ignore it. Simpler code, and it benefits any provider that supports session affinity (not just Fireworks).
  #   8. Thread the session_id from the caller into get_openai(). The session UUID is already available at all call sites (session.id in NAPI, uuid::Uuid::new_v4() in CLI). Add a session_id parameter to get_openai() and pass it through to from_api_key_with_options(). This is cleaner than env vars and consistent with create_rig_agent() which already takes session_id.
  #
  # EXAMPLES:
  #   1. User sets OPENAI_BASE_URL=https://api.fireworks.ai/inference and OPENAI_API_KEY=fw-xxx, starts a session — requests include x-session-affinity header with the session UUID, Fireworks routes all requests to the same replica, cache hit rate is high
  #   2. User sets OPENAI_BASE_URL=http://localhost:8888 (vLLM local server) — requests are sent normally, header may or may not be present (ignored by vLLM), no behavior change
  #   3. User sets OPENAI_SESSION_AFFINITY=my-custom-session — requests use 'my-custom-session' as the affinity value instead of the auto-generated session UUID
  #   4. OpenAI provider constructs rig client with custom base URL — the x-session-affinity header is included in the HeaderMap passed to http_headers(), propagated to both streaming and non-streaming requests
  #   5. Fireworks returns usage with prompt_tokens_details.cached_tokens — the value is captured in rig's cache_read_input_tokens and displayed in session token metrics
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we send x-session-affinity unconditionally for ALL OpenAI-compatible endpoints (harmless for non-Fireworks), or detect Fireworks specifically (e.g., check if base URL contains 'fireworks.ai')?
  #   A: Send unconditionally for all custom base URL endpoints. The header is harmless — servers that don't understand it will ignore it. Simpler code, and it benefits any provider that supports session affinity (not just Fireworks).
  #
  #   Q: Should session_id be threaded through from the caller (get_openai needs the session UUID), or should the provider read an env var set earlier by the session setup?
  #   A: Thread the session_id from the caller into get_openai(). The session UUID is already available at all call sites (session.id in NAPI, uuid::Uuid::new_v4() in CLI). Add a session_id parameter to get_openai() and pass it through to from_api_key_with_options(). This is cleaner than env vars and consistent with create_rig_agent() which already takes session_id.
  #
  # ========================================
  Background: User Story
    As a developer using Fireworks.ai models
    I want to have session affinity headers sent with API requests
    So that prompt cache hit rates are maximized, reducing latency and cost

  @unit
  Scenario: Session affinity header is set when using custom base URL
    Given OPENAI_BASE_URL is set to "https://api.fireworks.ai/inference"
    And OPENAI_API_KEY is set to "fw-test-key"
    And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    When an OpenAI provider is created with that session ID
    Then the rig client headers should include "x-session-affinity" with value "550e8400-e29b-41d4-a716-446655440000"

  @unit
  Scenario: Session affinity header uses custom value from environment
    Given OPENAI_BASE_URL is set to "https://api.fireworks.ai/inference"
    And OPENAI_API_KEY is set to "fw-test-key"
    And OPENAI_SESSION_AFFINITY is set to "my-custom-session"
    And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    When an OpenAI provider is created with that session ID
    Then the rig client headers should include "x-session-affinity" with value "my-custom-session"

  @unit
  Scenario: Session affinity header is sent for any custom base URL endpoint
    Given OPENAI_BASE_URL is set to "http://localhost:8888"
    And OPENAI_API_KEY is set to "test-key"
    And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    When an OpenAI provider is created with that session ID
    Then the rig client headers should include "x-session-affinity" with value "550e8400-e29b-41d4-a716-446655440000"

  @unit
  Scenario: No session affinity header when using default OpenAI API
    Given OPENAI_BASE_URL is not set
    And OPENAI_API_KEY is set to "sk-test-key"
    And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    When an OpenAI provider is created with that session ID
    Then the rig client headers should not include "x-session-affinity"

  @unit
  Scenario: get_openai accepts session_id parameter
    Given a provider manager with OpenAI credentials
    And a session with UUID "550e8400-e29b-41d4-a716-446655440000"
    When get_openai is called with the session ID
    Then the returned provider should have the session affinity header set

  @unit
  Scenario: Cached tokens from Fireworks response are captured in usage metrics
    Given an OpenAI completion response with prompt_tokens_details.cached_tokens of 5000
    When the response is deserialized
    Then the usage should report cache_read_input_tokens as 5000
