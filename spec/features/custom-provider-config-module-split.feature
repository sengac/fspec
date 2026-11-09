@done
@REFAC-030
@providers
@refactor
@code-quality
Feature: Custom-provider config, management and limit tests split into small modules
  """
  rust/providers/src/custom/config.rs keeps ModelDef, Defaults, SystemPromptConfig and
  ProviderConfig; AuthConfig moves to config/auth.rs and ToolStyle/ApiStyle to
  config/styles.rs, re-exported from config. rust/providers/src/custom/management.rs
  keeps the info structs, list_providers_info and show_provider_info plus the private
  helpers its children share; config validation, connection testing and templates move
  to management/provider_files.rs and the facade/env helpers to management/facade_env.rs,
  re-exported from management. The four oversized test targets keep their top-level
  files and load their test functions from #[path] submodules. Coverage mappings into
  the moved lines are re-pointed.

  PROV-170: a model that omits its limits now reports them unavailable (None)
  instead of the guessed 200000/4096 defaults.
  """

  Background: User Story
    As an fspec developer
    I want custom-provider config, management and their limit tests split into small files
    So that PROV-170 changes to model limits touch files under 300 lines

  Scenario: Custom-provider config, management and limit test files stay under 300 lines
    Given the custom-provider config and management sources and the four limit test targets in rust/providers
    When the line count of each file and of every file split from it is measured
    Then no file has 300 or more lines

  Scenario: The custom-provider config and management surface is unchanged
    Given code that names ModelDef, AuthConfig, ProviderConfig and ProviderModelInfo through codelet_providers::custom
    When a provider config that omits model limits and auth is parsed
    Then its model reports no context window and no max output tokens and its auth is the custom variant
