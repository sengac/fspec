@done
@PROV-145
@rust
@rpc
@provider-settings
@high
Feature: Per-profile Loop Detection wire schema
  """
  The wire-portable ProfileDefinition (rust/rpc-types/src/lib.rs) carries four
  flat optional fields — `loop_detection_enabled: Option<bool>`,
  `loop_detection_window: Option<u32>`,
  `loop_detection_max_repeats: Option<u32>`,
  `loop_detection_max_retries: Option<u32>` (snake_case on the wire; camelCase
  loopDetectionEnabled / loopDetectionWindow / loopDetectionMaxRepeats /
  loopDetectionMaxRetries on disk) so the `napi(object)` projection stays a
  plain struct, mirroring the PROV-142 `auto_continue` / PROV-144
  `max_images` fields. Canonical predicates on ProfileDefinition are the
  single source of truth for the effective values: enabled resolves to true
  when absent (preserving today's always-on behavior); window / maxRepeats /
  maxRetries resolve to the stored value or the RIG-014 defaults 160 / 10 /
  10. A legacy config file without the keys still deserializes (all fields
  default to None).
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want loop-detection settings on the profile's wire shape
    So that the effective detector configuration resolves canonically (absent => RIG-014 defaults)

  # ========================================
  # WIRE SCHEMA
  # ========================================
  Scenario: The loop-detection values round-trip through the wire JSON shape
    Given a profile definition with loopDetectionEnabled true, loopDetectionWindow 320, loopDetectionMaxRepeats 5, loopDetectionMaxRetries 2
    When the profile definition is serialized to its wire JSON form
    Then the JSON carries the loop_detection_enabled, loop_detection_window, loop_detection_max_repeats, and loop_detection_max_retries values
    And re-deserializing the JSON yields the same four values
    And the canonical predicates resolve the effective values to 320, 5, 2 and enabled

  Scenario: An absent loopDetectionEnabled key resolves to enabled
    Given a legacy config file without the loopDetectionEnabled key
    When it is deserialized into a profile definition
    Then the loop_detection_enabled field is absent
    And the canonical enabled predicate resolves to true (today's always-on behavior)

  Scenario: An explicit loopDetectionEnabled false resolves to disabled
    Given a profile definition with loopDetectionEnabled false
    When the canonical enabled predicate is applied
    Then it resolves to false

  Scenario: Absent numeric keys resolve to the RIG-014 defaults
    Given a profile definition without the loop-detection numeric fields
    When the canonical predicates are applied
    Then the effective window resolves to 160
    And the effective maxRepeats resolves to 10
    And the effective maxRetries resolves to 10
