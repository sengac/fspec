@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Provider runtime settings resolution
  """
  Shared optional settings resolve field by field for the logical provider. Independent token precedence is session, model, profile, provider, models.dev, backend fallback. Catalog aliases do not change settings ownership. Historical Anthropic values are fallbacks, not universal ceilings.
  """

  Background: User Story
    As a developer configuring fspec providers
    I want automatic model limits and explicit runtime overrides
    So that every selection path resolves the same effective settings

  Scenario: Runtime settings inherit field by field
    Given provider defaults and a selected profile with omitted fields and explicit false values
    When runtime settings are resolved for a selected model
    Then model overrides precede profile overrides and provider defaults without losing explicit false

  Scenario: Catalog token defaults apply to every selection path
    Given matching models.dev context and output limits with no explicit overrides
    When ordinary direct and profile-qualified models are selected
    Then all selections use the same catalog context and output limits

  Scenario: Logical providers retain settings ownership across facades
    Given Codex and a custom provider use another provider protocol or catalog alias
    When their model limits and runtime options are resolved
    Then catalog aliases supply metadata without borrowing another logical provider settings

  Scenario: Token overrides resolve independently and clear to automatic
    Given provider profile and model token overrides with matching catalog metadata
    When each override is applied and independently cleared
    Then each limit follows model profile provider catalog and backend fallback precedence independently

  Scenario: Anthropic catalog limits are not capped by historical defaults
    Given an Anthropic model with catalog context and output limits above historical defaults
    When the model is selected without explicit overrides
    Then the resolved limits retain catalog values rather than blanket 200000 and 8192 ceilings
