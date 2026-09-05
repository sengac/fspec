@done
@PROV-145
@rust
@persistence
@providers
@high
Feature: Per-profile Loop Detection persistence
  """
  The four loop-detection values round-trip through the on-disk ProfileDef
  (rust/sessions/src/profile_persistence.rs) and the wire-to-disk bridge
  profile_def_from_wire (rust/sessions/src/conversions.rs). save_profile_at /
  rename_profile_at write the flat camelCase keys loopDetectionEnabled /
  loopDetectionWindow / loopDetectionMaxRepeats / loopDetectionMaxRetries via
  the established set-or-remove pattern: Some(v) writes the key (including
  `"loopDetectionEnabled": false`); None REMOVES the key so an absent key
  continues to mean the RIG-014 default (enabled => ON, window 160,
  maxRepeats 10, maxRetries 10). The disk read path
  (rust/sessions/src/profile_sections.rs LocalServerProfile) deserializes the
  numeric keys with the shared lenient u32 deserializer so a TS-written
  float (e.g. 320.0) saturates rather than dropping the whole profile.
  Pre-existing profiles without the keys keep working unchanged.
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want my loop-detection settings persisted per profile
    So that an absent key always means the RIG-014 default and an explicit value always wins

  # ========================================
  # PERSISTENCE: wire <-> disk round trip
  # ========================================
  Scenario: The loop-detection values round-trip through wire and disk
    Given a profile definition with loopDetectionEnabled true, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    When the profile is saved to fspec-config.json
    Then the stored profile object contains "loopDetectionEnabled": true, "loopDetectionWindow": 320, "loopDetectionMaxRepeats": 5, "loopDetectionMaxRetries": 2
    And re-reading the profile resolves the effective values to 320, 5, 2 and enabled

  Scenario: An explicit loopDetectionEnabled false is written and read back
    Given a profile definition with loopDetectionEnabled false and no loop-detection numeric fields
    When the profile is saved to fspec-config.json
    Then the stored profile object contains "loopDetectionEnabled": false
    And the stored profile object has no loopDetectionWindow, loopDetectionMaxRepeats, or loopDetectionMaxRetries keys
    And re-reading the profile resolves the effective detector state to disabled with default window 160, maxRepeats 10, maxRetries 10

  Scenario: Saving without loop-detection values removes the stored keys
    Given a profile "work-vllm" previously stored loopDetectionWindow 320 and loopDetectionMaxRetries 2
    When the profile is saved with no loop-detection values
    Then the stored profile object has no loopDetectionEnabled, loopDetectionWindow, loopDetectionMaxRepeats, or loopDetectionMaxRetries keys
    And re-reading the profile resolves every effective value to its RIG-014 default (enabled, 160, 10, 10)

  Scenario: Renaming a profile carries the loop-detection values
    Given a profile "work-vllm" stores loopDetectionWindow 320 and loopDetectionMaxRetries 2
    When the profile is renamed to "home" with the same values
    Then the "home" profile object contains the stored loop-detection values
    And the "work-vllm" key no longer exists

  Scenario: A legacy profile without the loop-detection keys loads unchanged
    Given a pre-existing profile object that carries only baseUrl and apiKey
    When the local-server profiles are loaded from fspec-config.json
    Then the profile loads with every loop-detection field absent
    And re-reading resolves the effective values to the RIG-014 defaults (enabled, 160, 10, 10)

  Scenario: A TS-written float loop-detection value saturates on read
    Given a stored profile whose loopDetectionWindow value is the float 320.0
    When the local-server profiles are loaded from fspec-config.json
    Then the profile loads and its loopDetectionWindow is 320
