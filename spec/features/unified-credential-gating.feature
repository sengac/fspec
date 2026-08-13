@done
@PROV-128
@providers
@model-selection
@ts-parity
Feature: Unify credential gating between section availability and model population (TS parity)
  """
  Fix location: rust/sessions/src/cloud_models.rs::provider_has_credentials — after the existing API-key resolution chain (resolve_credential: credentials file -> env -> project .env), add an OAuth-parity fallback that reads the SAME codelet_providers::ProviderCredentials::detect() the header-availability flag uses, returning has_claude()/has_codex()/has_github_copilot() for anthropic/codex/github-copilot. This makes the population gate and the header flag one credential decision (single source of truth).
  The header-availability flag is rust/providers/src/custom/management.rs::list_providers_info (has_openai/has_codex/has_github_copilot via ProviderCredentials::detect). The model-population gate is handle_impl.rs::list_providers -> provider_has_credentials. Out of scope: openai->codex re-parenting/allowlist (PROV-129) and ordering (PROV-130). No unwrap/expect/panic in the production path.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Credential resolution honors OAuth for every OAuth-capable provider (anthropic, codex, github-copilot), not anthropic only
  #   2. A provider's header-availability flag and its model-population gate read from the same credential decision, so there are no credentialed-but-empty sections
  #   3. A provider with no credential of any kind (no API key, no OAuth) resolves to 'no credential' in BOTH the header flag and the population gate
  #
  # EXAMPLES:
  #   1. With a Codex OAuth file present and no OPENAI_API_KEY, the model-population gate reports the codex provider as credentialed (models can populate) — matching its header availability flag
  #   2. With a GitHub Copilot OAuth token present and no API key, the model-population gate reports github-copilot as credentialed — matching its header availability flag
  #   3. With no credentials of any kind for codex/github-copilot, both the population gate and the header availability flag agree the provider is uncredentialed (so it is dropped, never a dead header)
  #   4. With a Claude OAuth file present and no ANTHROPIC_API_KEY, the population gate reports anthropic as credentialed (already true before this card; preserved)
  #
  # ========================================
  Background: User Story
    As a codelet TUI user with an OAuth-only provider (Codex or GitHub Copilot)
    I want to have the model selector gate a provider's models by the same credential decision its header uses
    So that I never see a credentialed provider header that renders zero models — it either populates or is dropped consistently

  @server
  Scenario: Codex OAuth alone satisfies the model-population gate
    Given a Codex OAuth credential file is present
    And no OPENAI_API_KEY is set in the environment
    When the model-population gate is evaluated for the "codex" provider
    Then the gate reports "codex" as credentialed
    And the gate decision equals the header availability flag for "codex"

  @server
  Scenario: GitHub Copilot OAuth alone satisfies the model-population gate
    Given a GitHub Copilot OAuth token file is present
    And no Copilot API key is set in the environment
    When the model-population gate is evaluated for the "github-copilot" provider
    Then the gate reports "github-copilot" as credentialed
    And the gate decision equals the header availability flag for "github-copilot"

  @server
  Scenario: No credentials means both the gate and the header agree the provider is uncredentialed
    Given no credential of any kind exists for "codex" or "github-copilot"
    When the model-population gate is evaluated for each provider
    Then the gate reports each provider as uncredentialed
    And the header availability flag reports each provider as uncredentialed

  @server
  Scenario: Claude OAuth continues to satisfy the anthropic gate
    Given a Claude OAuth credential file is present
    And no ANTHROPIC_API_KEY is set in the environment
    When the model-population gate is evaluated for the "anthropic" provider
    Then the gate reports "anthropic" as credentialed
    And the gate decision equals the header availability flag for "anthropic"
