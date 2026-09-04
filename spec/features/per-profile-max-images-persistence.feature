@done
@PROV-144
@rust
@persistence
@providers
@high
Feature: Per-profile Max Images persistence
  """
  The maxImages value round-trips through the on-disk ProfileDef
  (rust/sessions/src/profile_persistence.rs) and the wire-to-disk bridge
  profile_def_from_wire (rust/sessions/src/conversions.rs). save_profile_at /
  rename_profile_at write the `maxImages` key via the established
  set-or-remove pattern: Some(n) writes `"maxImages": n` (including the
  explicit `0` no-vision sentinel); None REMOVES the key so an absent key
  continues to mean the default 4. The disk read path
  (rust/sessions/src/profile_sections.rs LocalServerProfile) deserializes
  `maxImages` with the shared lenient u32 deserializer so a TS-written
  float (e.g. 2.0) saturates rather than dropping the whole profile.
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want my Max Images value persisted per profile
    So that an absent key always means the default 4 and an explicit 0 always means no vision

  # ========================================
  # PERSISTENCE: wire <-> disk round trip
  # ========================================

  Scenario: The maxImages value round-trips through wire and disk
    Given a profile definition with maxImages 7
    When the profile is saved to fspec-config.json
    Then the stored profile object contains "maxImages": 7
    And re-reading the profile resolves the effective limit to 7

  Scenario: A missing maxImages key resolves to the default 4
    Given a profile definition without a maxImages field
    When the profile is saved to fspec-config.json
    Then the stored profile object has no maxImages key
    And re-reading the profile resolves the effective limit to 4
