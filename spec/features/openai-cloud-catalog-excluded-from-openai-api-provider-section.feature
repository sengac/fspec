@done
@PROV-146
@bug
@model-selection
@providers
@openai
Feature: OpenAI cloud catalog excluded from OpenAI API provider section
  """
  Fix location: rust/sessions/src/handle_impl.rs list_providers() — the RPC-073 cloud-population loop (for each built-in provider, fill empty model lists from the models.dev registry gated on provider_has_credentials). That loop must skip the 'openai' provider: the standalone OpenAI API cloud section is NEVER populated from the models.dev catalog. It keeps its zero-model empty list, which the existing PROV-127 retain_populated_cloud_sections filter then drops, so it renders no '(0 models)' header.
  Cloud OpenAI models continue to surface EXCLUSIVELY via the existing Codex re-parenting path: synthesize_codex_section() (PROV-129/PROV-130) calls codex_reparented_models() → cloud_model_entries(registry, 'openai', true) unconditionally (Codex-gated), allowlist-filtered, under the 'Codex (ChatGPT)' section. That path is unchanged.
  The exclusion is a call-site concern in list_providers, NOT inside cloud_model_entries() — the pure helper keeps its generic contract (rpc073/cloud-model-catalog tests assert it serves 'openai' when asked).
  Local-server profile sections (openai:<profile>, built by build_local_profile_sections) are a separate bucket appended AFTER the drop-empty filter and are completely unaffected — they still probe /v1/models and merge customModels.
  Regression guard: a profile's apiKey bridged into OPENAI_API_KEY by apply_profile_env_vars (PROV-121) must no longer leak into the OpenAI API cloud section — the gate is bypassed for openai regardless of credential provenance.
  Supersedes (reverses) the PROV-129 scenario 'Without Codex credentials the standalone OpenAI API section is preserved' and the rpc-073-list-providers-wiring scenarios asserting a populated 'openai' cloud section: those codified the bug.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The standalone 'openai' (OpenAI API) cloud section must NEVER be populated from the models.dev cloud catalog in list_providers — regardless of whether OPENAI_API_KEY is set, where it came from (user env, profile bridge, credentials file), or whether a local profile exists. The openai provider is for local OpenAI-protocol-compatible servers (vLLM, Ollama, sglang, RunPod, Fireworks) only.
  #   2. Cloud OpenAI models (GPT-5.x, o3, GPT-4o etc. from the models.dev catalog) belong EXCLUSIVELY under the 'Codex (ChatGPT)' provider — sourced via the existing Codex re-parenting path (synthesize_codex_section / codex_reparented_models) which reads the OpenAI catalog unconditionally when Codex credentials are present.
  #   3. Local-server openai profile sections (openai:<profile>) are unaffected and must continue to render with their probed /v1/models + customModels entries; the fix only suppresses the standalone cloud 'openai' section, which is dropped by the PROV-127 drop-empty filter since it now always carries zero models.
  #   4. The exclusion is applied at the list_providers() cloud-population call site, NOT inside cloud_model_entries() — because the Codex re-parenting path (codex_reparented_models) legitimately calls cloud_model_entries(registry, 'openai', true) to source the cloud OpenAI catalog. Non-openai cloud providers (anthropic, gemini, etc.) still populate from the catalog unchanged.
  #
  # EXAMPLES:
  #   1. User has a sglang profile (baseUrl localhost:18003, apiKey 'test') and runs a session on it; OPENAI_API_KEY=test is bridged into env. Opening /model shows 'openai: sglang' with the profile's models, and NO 'OpenAI API' section with GPT-5.6 Terra/Luna/Sol, GPT-5.5, o3, GPT-4o — the 30-model cloud catalog does not appear under OpenAI API.
  #   2. User sets a genuine cloud OPENAI_API_KEY in their shell (no profiles, no Codex creds): the 'OpenAI API' section still shows NO cloud catalog models — the openai cloud section is suppressed entirely (the user picks local-server models via profiles instead, and cloud GPT models are available under Codex if they log in there).
  #   3. User is signed in with Codex (ChatGPT) OAuth AND has local openai profiles: /model shows the profiles (openai: sglang, openai: runpod.io) and the 'Codex (ChatGPT)' section with allowlisted cloud models (GPT-5.4, GPT-5.2-codex...), but no standalone 'OpenAI API' cloud section at all.
  #
  # ========================================
  Background: User Story
    As a fspec user running local OpenAI-protocol servers (vLLM, Ollama, sglang, RunPod, Fireworks) via profiles
    I want the OpenAI API provider section to list only my local-server models
    So that I never see the cloud OpenAI catalog (GPT-5.x, o3, GPT-4o) under 'OpenAI API' — cloud OpenAI models belong exclusively under Codex (ChatGPT)

  @server
  Scenario: OpenAI cloud catalog is not listed under the OpenAI API provider when a profile bridges OPENAI_API_KEY
    Given a local-server openai profile named "sglang" is configured with a baseUrl and an apiKey
    And the OPENAI_API_KEY environment variable is set to the profile apiKey value
    And the models.dev catalog offers OpenAI models including "o3" and "gpt-4o"
    When list_providers() assembles the provider list
    Then no section with key "openai" and no profile name appears in the list
    And the local-server profile section "openai:sglang" is still present with its models
    And no model with id "o3" appears in any non-Codex section

  @server
  Scenario: A raw OPENAI_API_KEY without profiles still yields no OpenAI API cloud section
    Given no local-server openai profiles are configured
    And only an OPENAI_API_KEY is set in the environment
    And an ANTHROPIC_API_KEY is also set in the environment
    And no Codex credentials are present
    And the models.dev catalog offers OpenAI models
    When list_providers() assembles the provider list
    Then no section with key "openai" and no profile name appears in the list
    And no "Codex (ChatGPT)" section is synthesized
    And the other credentialed cloud providers still list their catalog models

  @server
  Scenario: Codex (ChatGPT) continues to list the allowlisted cloud OpenAI models when OPENAI_API_KEY is also set
    Given I am signed in with a Codex (ChatGPT) OAuth credential
    And an OPENAI_API_KEY is set in the environment
    And a local-server openai profile is configured
    And the models.dev catalog offers the OpenAI models "gpt-5.4" and "gpt-5-mini"
    When list_providers() assembles the provider list
    Then the "Codex (ChatGPT)" section lists "gpt-5.4"
    And the "Codex (ChatGPT)" section does not list "gpt-5-mini"
    And no section with key "openai" and no profile name appears in the list
    And the local-server profile section is still present

  @unit
  Scenario: cloud_model_entries still serves the openai catalog to the Codex re-parenting path
    Given a models.dev registry where "openai" lists a tool_call model "gpt-5.4"
    When cloud_model_entries is built for "openai" with credentials asserted
    Then the entries contain "gpt-5.4"
    And the helper contract is unchanged (the exclusion is a list_providers call-site rule, not a helper rule)
