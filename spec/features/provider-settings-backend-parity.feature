@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Provider settings backend parity
  """
  Provider-qualified backend reads and writes reuse ProfileDefinition. Explicit defaults live under providers.<id>.settings. Legacy flat profiles and unrelated configuration survive edits. Profiles without connection overrides inherit provider authentication and discovery.
  """

  Background: User Story
    As a developer configuring fspec providers
    I want backend-authoritative settings and provider-scoped profiles
    So that configuration round trips without changing authentication

  Scenario: Provider settings round trip without changing credentials
    Given saved built-in and custom provider credentials and unrelated configuration
    When shared runtime defaults are saved and read through the backend
    Then explicit settings round trip while credentials and unrelated configuration remain unchanged

  Scenario: Named profile CRUD is provider scoped
    Given Codex and Claude each have a profile named careful
    When the Codex profile is renamed and deleted through the backend
    Then the Claude profile and its inherited authentication remain unchanged

  Scenario: Legacy OpenAI profiles remain readable
    Given an existing OpenAI profile with connection fields and runtime overrides
    When provider settings are read and the profile is edited
    Then legacy values and unrelated profile fields are preserved

  Scenario: Inherited authentication profiles are selectable
    Given a non-OpenAI provider has a named runtime profile without a discovery endpoint
    When available model sections are loaded
    Then the profile exposes the provider models without requiring an OpenAI discovery probe
