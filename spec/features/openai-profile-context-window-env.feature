@done
@providers
@model-selection
@rust
@BUG-341
Feature: OpenAI profile context window environment
  """
  apply_profile_env_vars (rust/sessions/src/profile_environment.rs) exports a
  selected OpenAI profile's connection settings as OPENAI_* variables, and
  since PROV-171 OPENAI_CONTEXT_WINDOW is the OpenAI resolver's context window
  default (ProviderManager snapshots it at selection). BUG-341: a profile that
  declares no contextWindow removes the variable, as OPENAI_STREAMING already
  is, so a profile never inherits the window of the profile selected before
  it. Tests: rust/sessions/tests/bug341_profile_context_window_env.rs.
  """

  Background: User Story
    As a user switching between local-server OpenAI profiles
    I want each profile to use only the context window it declares
    So that a model is never budgeted against another profile's limit

  Scenario: A profile that declares a context window exports it
    Given a stored OpenAI profile "big" whose contextWindow is 200000
    When the profile environment variables are applied for "big"
    Then OPENAI_CONTEXT_WINDOW is "200000"

  Scenario: Switching to a profile without a context window clears the previous one
    Given a stored OpenAI profile "big" whose contextWindow is 200000 and a profile "small" with no contextWindow
    And the profile environment variables were applied for "big"
    When the profile environment variables are applied for "small"
    Then OPENAI_CONTEXT_WINDOW is not set
