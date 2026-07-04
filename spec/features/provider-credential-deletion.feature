@done
@config-management
@providers
@PROV-133
Feature: Pressing 'd' in provider settings does not actually remove credentials

  """
  Authoritative multi-source credential delete (Option A). backend.delete_provider_credentials (sessions/handle_impl.rs:1383) must clear EVERY source the availability projection reads (management.rs:114-143 list_providers_info -> ProviderCredentials::detect, credentials.rs:103-236). Three sources: (1) credentials.json via credentials::delete_credential/writer.rs:106-119 (already done); (2) the process env var(s) — add resolver::remove_provider_env_vars(provider_id) mirroring update_all_provider_env_vars (resolver.rs:240-249), removing every name from get_provider_env_vars (resolver.rs:16-38) plus CLAUDE_CODE_OAUTH_TOKEN for anthropic; (3) the OAuth auth file for anthropic/codex/github-copilot — reuse copilot::auth::delete_copilot_auth (auth.rs:221) and add fs::remove_file for get_claude_auth_path()/codex get_auth_path(). Missing file/absent provider = no-op success. Only the targeted provider is affected; confirm-dialog + dispatch wiring unchanged. Tested via redirected $HOME/FSPEC_HOME temp home with a seeded credentials.json, env var, and fake auth file; assert list_provider_credentials reports configured=false for the target and unchanged for others.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. For an API-key provider, confirming delete must unset the process env var for that provider AND clear its credentials.json entry, so the next list_provider_credentials reports configured=false
  #   2. For an OAuth provider (anthropic/codex/github-copilot), confirming delete must remove the matching auth file (claude_auth.json / .codex/auth.json / copilot_auth.json) so detect() reports the provider unconfigured
  #   3. Delete only affects the targeted provider: other providers' env vars, credentials.json entries and OAuth auth files are left untouched
  #   4. The confirm-dialog and dispatch wiring (d -> open_delete_confirm -> ConfirmDeleteProviderCredentials -> handle_delete_provider_credentials -> backend.delete_provider_credentials) is unchanged; only the backend delete becomes authoritative across all credential sources
  #   5. Partially. Only github-copilot has a ready delete helper: providers/src/copilot/auth.rs:221 delete_copilot_auth() (async, remove_file(get_copilot_auth_path())). claude_auth.rs and codex/codex_auth.rs expose only get_claude_auth_path()/get_auth_path() + read/write — NO delete. So the fix reuses delete_copilot_auth for copilot and adds fs::remove_file(get_claude_auth_path()) for anthropic and fs::remove_file(get_auth_path()) for codex (missing-file = no-op success).
  #   6. Yes, it is the mirror of the existing set path. resolver.rs already mutates process-global env via std::env::set_var (resolve_and_set_env_var:196/201, update_all_provider_env_vars:245). The unset must remove ALL env var names for the provider from get_provider_env_vars (resolver.rs:16-38), e.g. openai->[OPENAI_API_KEY], gemini->[GOOGLE_GENERATIVE_AI_API_KEY,GEMINI_API_KEY], zai->[ZAI_API_KEY,ZAI_PLAN_API_KEY]. For anthropic also remove CLAUDE_CODE_OAUTH_TOKEN (set at resolver.rs:196). Add a resolver remove_provider_env_vars(provider_id) that calls std::env::remove_var over that list, complementing update_all_provider_env_vars.
  #   7. Safe as the exact mirror of the existing set_var path. The provider->env-var-names mapping is get_provider_env_vars (resolver.rs:16-38): openai->[OPENAI_API_KEY]; gemini->[GOOGLE_GENERATIVE_AI_API_KEY,GEMINI_API_KEY]; zai->[ZAI_API_KEY,ZAI_PLAN_API_KEY]; huggingface->[HUGGINGFACE_API_KEY,HF_TOKEN]; anthropic->[ANTHROPIC_API_KEY] plus CLAUDE_CODE_OAUTH_TOKEN (set at resolver.rs:196). Remove ALL names for the targeted provider only (never other providers'). Integration tests will set the var then assert std::env::var(...) is Err after delete.
  #
  # EXAMPLES:
  #   1. OpenAI configured via OPENAI_API_KEY env var: press d, confirm -> env var unset and credentials.json entry cleared -> next list shows OpenAI (not configured)
  #   2. Anthropic configured via OAuth in claude_auth.json: press d, confirm -> claude_auth.json removed -> Anthropic reports configured=false
  #   3. Delete Anthropic (OAuth) while Codex is also configured via its own auth.json: after confirming, Anthropic becomes unconfigured but Codex remains configured
  #
  # QUESTIONS (ANSWERED):
  #   Q: @self: Does an existing OAuth delete/logout helper already exist to reuse, per provider?
  #   A: Yes, it is the mirror of the existing set path. resolver.rs already mutates process-global env via std::env::set_var (resolve_and_set_env_var:196/201, update_all_provider_env_vars:245). The unset must remove ALL env var names for the provider from get_provider_env_vars (resolver.rs:16-38), e.g. openai->[OPENAI_API_KEY], gemini->[GOOGLE_GENERATIVE_AI_API_KEY,GEMINI_API_KEY], zai->[ZAI_API_KEY,ZAI_PLAN_API_KEY]. For anthropic also remove CLAUDE_CODE_OAUTH_TOKEN (set at resolver.rs:196). Add a resolver remove_provider_env_vars(provider_id) that calls std::env::remove_var over that list, complementing update_all_provider_env_vars.
  #
  #   Q: @self: Is unsetting the env var safe given it is process-global, and which env var names must be removed per provider?
  #   A: Safe as the exact mirror of the existing set_var path. The provider->env-var-names mapping is get_provider_env_vars (resolver.rs:16-38): openai->[OPENAI_API_KEY]; gemini->[GOOGLE_GENERATIVE_AI_API_KEY,GEMINI_API_KEY]; zai->[ZAI_API_KEY,ZAI_PLAN_API_KEY]; huggingface->[HUGGINGFACE_API_KEY,HF_TOKEN]; anthropic->[ANTHROPIC_API_KEY] plus CLAUDE_CODE_OAUTH_TOKEN (set at resolver.rs:196). Remove ALL names for the targeted provider only (never other providers'). Integration tests will set the var then assert std::env::var(...) is Err after delete.
  #
  # ========================================

  Background: User Story
    As a user of the Rust TUI provider settings view
    I want to press 'd' on a provider and have the credential actually removed so the provider becomes unconfigured
    So that I can truly remove credentials instead of a delete that silently does nothing

  Scenario: Deleting an API-key provider unsets its env var and clears credentials.json
    Given OpenAI is configured via the OPENAI_API_KEY environment variable
    And OpenAI has an entry in credentials.json
    When I confirm delete for OpenAI
    Then the OPENAI_API_KEY environment variable is unset
    And OpenAI has no entry in credentials.json
    And the provider list reports OpenAI as not configured

  Scenario: Deleting an OAuth provider removes its auth file
    Given Anthropic is configured via OAuth tokens in claude_auth.json
    When I confirm delete for Anthropic
    Then the claude_auth.json auth file is removed
    And the provider list reports Anthropic as not configured

  Scenario: Deleting one provider leaves other configured providers untouched
    Given Anthropic is configured via OAuth tokens in claude_auth.json
    And Codex is configured via OAuth tokens in its own auth.json
    When I confirm delete for Anthropic
    Then the provider list reports Anthropic as not configured
    And the provider list still reports Codex as configured

  Scenario: Deleting a provider with no stored credential is a safe no-op
    Given OpenAI has no environment variable, credentials.json entry, or auth file
    When I confirm delete for OpenAI
    Then the delete succeeds without error
    And the provider list reports OpenAI as not configured

