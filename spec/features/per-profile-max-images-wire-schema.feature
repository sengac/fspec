@done
@PROV-144
@rust
@rpc
@provider-settings
@high
Feature: Per-profile Max Images wire schema
  """
  The wire-portable ProfileDefinition (rust/rpc-types/src/lib.rs) carries a
  single flat `Option<u32>` field `max_images` (camelCase `maxImages` on
  disk) so the `napi(object)` projection stays a plain struct, mirroring
  the PROV-142 `auto_continue` field. The canonical predicate
  `max_images_limit()` is the single source of truth for the effective
  limit: `None` (key absent on disk, including pre-existing profiles)
  resolves to the default of 4; `Some(0)` is the explicit no-vision
  sentinel (never coerced to the default); `Some(n)` with `n >= 1` is a
  cap of `n` images per Read tool result. A legacy config file without the
  key still deserializes (the field defaults to None).
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want a Max Images limit on the profile's wire shape
    So that the effective per-Read image budget resolves canonically (absent => 4)

  # ========================================
  # WIRE SCHEMA
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
