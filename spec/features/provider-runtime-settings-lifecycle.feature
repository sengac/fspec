@PROV-161
@providers
@provider-settings
@configuration
@phase-1
Feature: Provider runtime settings lifecycle
  """
  Creation, resume, subordinate creation and model switching share effective settings resolution. Switches refresh inherited runtime defaults but preserve explicit session continuation choices. Header caches, tool image budgets and manager limits agree.
  """

  Background: User Story
    As a developer running fspec sessions
    I want saved provider settings applied throughout the session lifecycle
    So that model changes do not leave stale runtime settings

  Scenario: Session lifecycle applies current provider runtime settings
    Given saved provider settings for continuation images loop detection and thinking preservation
    When sessions are created resumed and spawned as subordinates
    Then each session applies current saved runtime settings and effective token limits

  Scenario: Model switching replaces inherited settings without losing explicit session choices
    Given an active session with inherited provider settings and an explicit continuation choice
    When the session switches to another provider or profile
    Then inherited settings and budgets refresh while the explicit continuation choice is preserved
