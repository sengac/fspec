@PROV-142
@wip
@rust
@persistence
@providers
Feature: Per-profile autoContinue persistence

  """
  The autoContinue value round-trips through the on-disk ProfileDef
  (rust/sessions/src/profile_persistence.rs) and the wire-to-disk bridge
  profile_def_from_wire (rust/sessions/src/conversions.rs). save_profile_at
  writes the camelCase "autoContinue" key into ~/.fspec/fspec-config.json via
  a read-modify-write: Some(0) is written as the explicit-off sentinel and
  None removes the key (absent ⇒ off, today's behavior). The read path
  (LocalServerProfile, rust/sessions/src/profile_sections.rs) deserializes the
  key leniently. Verified by
  rust/sessions/tests/prov142_auto_continue_persistence.rs.
  """

  Background: User Story
    As a developer using a local OpenAI-compatible server profile
    I want the autoContinue value persisted without clobbering my other profile settings
    So that the auto-continue default survives a save and reload

  Scenario: The wire-to-disk bridge copies the autoContinue value
    Given a wire profile definition whose autoContinue value is 300
    When it is converted to the on-disk profile definition
    Then the on-disk definition carries autoContinue set to 300

  Scenario: Saving writes and removes the autoContinue key
    Given a stored profile that has no autoContinue key
    When the user types 0 and saves the profile (explicit off sentinel)
    Then the saved config file records the autoContinue key as 0
    And when the profile is saved again with no autoContinue value
    Then the saved config file no longer records the autoContinue key
