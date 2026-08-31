@done
@persistence
@provider-settings
@PROV-143
Feature: Profile preserve-thinking persistence (PROV-143)
  """
  architecture:
  - The wire ProfileDefinition.preserve_thinking bridges to the on-disk
  ProfileDef.preserve_thinking via profile_def_from_wire.
  - save_profile writes "preserveThinking": true|false when Some(_);
  None removes the key so an absent key continues to mean stripped.
  """

  Background: User Story
    As a provider profile user
    I want my Preserve Thinking choice to survive a restart
    So that sessions always use the toggle value I last saved

  Scenario: The wire-to-disk bridge copies the preserveThinking value
    Given a wire profile definition whose preserveThinking value is true
    When it is converted to the on-disk profile definition
    Then the on-disk definition carries preserveThinking set to true

  Scenario: Saving writes and removes the preserveThinking key
    Given a stored profile that has no preserveThinking key
    When the user enables Preserve Thinking and saves the profile
    Then the saved config file records the preserveThinking key as true
    And when the profile is saved again with no preserveThinking value
    Then the saved config file has no preserveThinking key
