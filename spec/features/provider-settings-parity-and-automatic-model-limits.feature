@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Provider settings parity and automatic model limits
  """
  Reuse optional shared runtime options and provider-qualified backend forms. Store explicit defaults under providers.<id>.settings; preserve legacy flat profiles. Independent precedence: session, model, profile, provider, models.dev, backend fallback. Codex aliases OpenAI catalog metadata but owns its settings and omits forbidden output parameters. Anthropic and Gemini preserve signed reasoning by default. Supersedes historical OpenAI-only profile restrictions. Profiles without connection overrides inherit authentication and discovery. Keep new modules below 300 lines. See spec/attachments/PROV-161/provider-settings-parity-research.md.
  """

  Background: User Story
    As a developer configuring fspec providers
    I want to configure shared runtime settings and automatic token limits for every provider
    So that provider-specific authentication and capabilities remain intact

  Scenario: Every provider exposes settings and named profiles
    Given built-in and custom providers are shown in provider settings
    When each provider is expanded
    Then each provider offers Settings and provider-scoped profile creation

  Scenario: OAuth profiles inherit authentication
    Given a Codex provider authenticated with OAuth
    When a named runtime profile is saved without connection fields
    Then the profile saves successfully without replacing OAuth authentication

  Scenario: Automatic token limits remain independent in the editor
    Given a provider settings form with an explicit context limit and automatic output limit
    When the context limit is cleared and the form is saved
    Then both token overrides are absent and the editor labels automatic limits as model-specific

  Scenario: Invalid token overrides cannot silently become automatic
    Given a provider settings form with an invalid token limit
    When the user attempts to save
    Then the form shows a validation error and emits no save action

  Scenario: Same named profiles remain isolated in the editor
    Given Codex and Claude each have a profile named careful with different settings
    When each profile is opened from backend-loaded provider settings
    Then each editor shows only its own provider settings
