@done
@session-creation
@providers
@rust
@bug
@PROV-141
Feature: Session creation fails on Linux when default model is a profile model and no global provider credentials exist
  """
  Fix location: rust/providers/src/manager.rs ProviderManager::with_model_support() (lines ~398-436). Remove the `if !credentials.has_any() { return Err(...) }` block. The credentials snapshot is still taken and stored (select_model re-detects on each call anyway). deferred_placeholder_provider already handles the no-credentials case (falls back to OpenAI placeholder). Session creation paths (create_session_with_id, create_session_from_manifest, create_isolated_session_with_id) all construct via with_model_support() then apply the explicit model via apply_model_selection / select_model / set_model_direct — per-model credential validation is preserved there.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderManager::with_model_support() must NOT reject construction when ProviderCredentials::detect().has_any() is false — a registry-backed manager makes no provider selection at construction; the explicit model applied immediately after is the selection.
  #   2. Credential validation remains per-model, not per-construction: cloud registry models are validated in select_model (which re-detects credentials and errors if the provider has none); profile models bridge their own apiKey/baseUrl from fspec-config.json into OPENAI_* env vars via apply_profile_env_vars; codex/custom models use their own credential paths. No global has_any() gate is required anywhere on the session-creation path.
  #   3. A cloud registry model selected without credentials for its provider must still fail loudly at selection time (select_model's has_credentials check), with an error naming the provider and listing available providers — the fix must not make uncredentialed cloud models silently succeed.
  #
  # EXAMPLES:
  #   1. On a fresh Linux machine with no provider API keys in the environment and no auth files, the user sets their default model to a local-server profile (openai:spark/qwen3.8-27b, whose profile stores its own baseUrl and apiKey). Starting a session succeeds and the session runs on that profile model — no 'No provider credentials found' error.
  #   2. On the same credential-less machine, the user sets their default model to a cloud registry model (anthropic/claude-opus-4-5). Starting a session still fails loudly with an error naming the provider and listing available providers — the fix does not make uncredentialed cloud models silently succeed.
  #   3. On the credential-less machine, the user sets their default model to a codex model (codex/gpt-5). Starting a session succeeds — codex models resolve their own credentials and never needed a global API key at manager construction.
  #
  # ========================================
  Background: User Story
    As a developer on a machine with no global provider credentials
    I want to create a session whose default model is a local-server profile model
    So that the session starts using the profile's own stored credentials without needing a global API key in the environment

  Scenario: Profile model session creation succeeds without global credentials
    Given a SessionManager with no provider credentials in the environment
    And a local-server profile "spark" stored with its own baseUrl and apiKey
    And the default model is set to "openai:spark/qwen3.8-27b"
    When I call create_session with no role
    Then the returned session id value is not empty
    And the created session model is "openai:spark/qwen3.8-27b"

  Scenario: Cloud registry model still fails without credentials for its provider
    Given a SessionManager with no provider credentials in the environment
    And the default model is set to "anthropic/claude-opus-4-5"
    When I call create_session with no role
    Then the returned session id value is empty
    And the error message names the provider "anthropic"

  Scenario: Codex model session creation succeeds without global credentials
    Given a SessionManager with no provider credentials in the environment
    And the default model is set to "codex/gpt-5"
    When I call create_session with no role
    Then the returned session id value is not empty
