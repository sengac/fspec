@done
@PROV-140
@rust
@providers
@provider-settings
Feature: OpenAI profile streaming flag env-var bridge

  """
  apply_profile_env_vars (codelet/sessions/src/model_resolution.rs) is the
  single source of truth that exports a selected profile's connection settings
  as OPENAI_* environment variables. It now also exports OPENAI_STREAMING from
  the loaded profile's streaming flag, mirroring the existing OPENAI_BASE_URL /
  OPENAI_API_KEY / OPENAI_CONTEXT_WINDOW exports. An absent flag leaves
  streaming enabled (the provider default).
  """

  Background: User Story
    As a fspec user who disabled streaming on an OpenAI profile
    I want my streaming choice carried to the provider like my other profile settings
    So that selecting the profile actually turns streaming off at runtime

  Scenario: Selecting a streaming-disabled profile exports OPENAI_STREAMING false
    Given a stored OpenAI profile whose streaming flag is disabled
    When the profile environment variables are applied for that profile
    Then the OPENAI_STREAMING environment variable is set to false

  Scenario: Selecting a profile without a streaming flag leaves streaming enabled
    Given a stored OpenAI profile with no streaming flag
    When the profile environment variables are applied for that profile
    Then the OPENAI_STREAMING environment variable does not force streaming off
