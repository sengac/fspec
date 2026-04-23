@BUG-125
Feature: Copilot Rust files exceed 300-line limit (PROV-053 rule 21)
  """
  Extracted token refresh orchestration lives in token_refresh.rs. provider.rs still under 300 lines.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. oauth.rs (364→93) and refreshing_client.rs (331→218) already fixed by prior refactoring
  #   2. provider.rs (357 lines) must be reduced to under 300 lines
  #   3. Token refresh orchestration (ensure_fresh_copilot_token) should live in token_refresh.rs alongside the pure decision helpers
  #   4. Convenience re-export associated functions (base_url_for, system_prompt_facade_for_endpoint, list_models) on CopilotProvider are redundant since mod.rs already re-exports the module-level functions
  #   5. All 386 Rust copilot-provider tests must continue to pass after refactoring
  #
  # EXAMPLES:
  #   1. Extract ensure_fresh_copilot_token from CopilotProvider into a free function in token_refresh.rs, leave thin delegate on struct
  #   2. Remove base_url_for, system_prompt_facade_for_endpoint, list_models convenience methods from CopilotProvider, update test callers to use module-level functions
  #   3. After refactoring, provider.rs is ~295 lines (under 300)
  #
  # ========================================
  Background: User Story
    As a developer
    I want to refactor provider.rs to be under 300 lines
    So that the codebase stays compliant with the 300-line-per-file rule (PROV-053 rule 21)

  Scenario: provider.rs is under 300 lines after refactoring
    Given the provider.rs file was previously 357 lines due to mixed concerns
    When the token refresh orchestration is extracted into token_refresh.rs and convenience re-export methods are removed
    Then provider.rs is under 300 lines
    Then all copilot-provider Rust tests pass

  Scenario: Token refresh orchestration is accessible via CopilotProvider delegate
    Given ensure_fresh_copilot_token has been extracted to a free function in token_refresh.rs
    When CopilotProvider.ensure_fresh_copilot_token() is called
    Then it delegates to the free function in token_refresh.rs with the auth RwLock

  Scenario: Callers use module-level functions instead of CopilotProvider convenience methods
    Given base_url_for, system_prompt_facade_for_endpoint, and list_models were convenience re-exports on CopilotProvider
    When the convenience methods are removed from CopilotProvider
    Then test callers import and use the module-level functions directly
    Then no compilation errors are introduced
