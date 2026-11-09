@BUG-582
@done
@REFAC-028
@providers
@refactor
@code-quality
Feature: ProviderManager and model-limit resolution split into small modules
  """
  rust/providers/src/manager.rs keeps ProviderType, the limits resolver stub and the
  ProviderManager struct; its inherent methods live in include!d `impl ProviderManager`
  fragments under rust/providers/src/manager/ and its unit tests in include!d fragments
  of the same `tests` module, so every item keeps its module path and privacy.
  rust/providers/src/model_limits.rs keeps the ModelLimitsResolver trait and the
  resolution functions; its three test suites become file modules under
  rust/providers/src/model_limits/ (the LIMITS-007 suite split into its scenarios and
  its edge cases). Coverage mappings into the moved lines are relocated.
  """

  Background: User Story
    As an fspec developer
    I want ProviderManager and model-limit resolution split into small files
    So that changes to provider limits touch files under 300 lines

  Scenario: ProviderManager and model-limit source files stay under 300 lines
    Given the ProviderManager and model-limit source files in rust/providers/src
    When the line count of each file is measured
    Then no file has 300 or more lines

  Scenario: The ProviderManager and model-limit surface is unchanged
    Given code that names ProviderManager, ProviderType and resolve_context_window through codelet_providers
    When a Claude manager with registry limits resolves its context window and output limit
    Then it reports the registry values
