@providers
@authentication
@integration
@PROV-094
Feature: Ship codex.rhai + codex.json replicating Codex ChatGPT subscription flow at parity
  """
  Ship opt-in templates at rust/providers/templates/codex. Reuse ScriptedOAuthProvider and RhaiCustomProvider; add base64url_decode to the sandbox crypto module and expose model plus system prefix in request context. Device flow only is sufficient per original and/or scope; no keychain or auto-install.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The optional bundle uses Codex device authorization and form-encoded token exchange, account JWT extraction, file credentials, native tool names and Responses requests; it never replaces the built-in provider automatically.
  #
  # EXAMPLES:
  #   1. A subscriber authorizes a device code, polls pending then success, stores tokens and refreshes them using a local OAuth issuer.
  #   2. The script constructs the same Responses request and subscription headers as the built-in provider for text, tool calls and tool outputs.
  #   3. Streaming text reasoning tool calls and usage reach the common stream bridge and failed responses remain errors.
  #
  # ========================================
  Background: User Story
    As a Codex subscriber I want to use a bundled provider script So that I can inspect authentication and Responses requests

  Scenario: Authorize and refresh a Codex subscription with file credentials
    Given the bundled Codex script and a local OAuth issuer
    When I request a device code authorize it and refresh the resulting tokens
    Then pending authorization and successful token exchange are distinguished
    And form encoded tokens account identity and file credentials are preserved

  Scenario: Build subscription requests at built-in provider parity
    Given a bundled Codex provider with text tool history and stored subscription tokens
    When I build a Responses request and its headers
    Then the body and subscription headers match the built-in provider
    And the bundle exposes Codex native tools and base instructions

  Scenario: Stream subscription output and reject failures
    Given the bundled script processes Responses events
    When text reasoning completed tools usage and failed events arrive
    Then the common stream bridge receives output and token counts without swallowing errors
