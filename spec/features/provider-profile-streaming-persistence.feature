@done
@PROV-139
@rust
@provider-settings
@persistence
Feature: OpenAI profile streaming flag persistence
  """
  The streaming flag round-trips through the on-disk ProfileDef
  (rust/sessions/src/profile_persistence.rs) and the wire-to-disk bridge
  profile_def_from_wire (rust/sessions/src/conversions.rs). save_profile_at
  writes the camelCase "streaming" key into ~/.fspec/fspec-config.json via a
  read-modify-write that preserves customModels and the compaction-threshold
  fields; a profile with no streaming key loads as enabled. Verified by
  rust/sessions/tests/prov139_streaming_persistence.rs.
  """

  Background: User Story
    As a fspec user configuring an OpenAI-compatible endpoint
    I want the streaming flag persisted without clobbering my other profile settings
    So that toggling streaming survives a save and reload

  Scenario: Saving preserves customModels while writing the streaming key
    Given a stored profile that has a custom model
    When the user disables Streaming and saves the profile
    Then the saved config file records the streaming key as disabled
    And the saved config file still lists the custom model

  Scenario: Loading a profile without a streaming key defaults to enabled
    Given a config file whose profile has no streaming key
    When the profile is loaded
    Then the loaded profile reports streaming as enabled

  Scenario: The wire-to-disk bridge copies the streaming flag
    Given a wire profile definition whose streaming flag is set to disabled
    When it is converted to the on-disk profile definition
    Then the on-disk definition carries streaming set to disabled
