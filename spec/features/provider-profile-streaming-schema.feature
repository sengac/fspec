@done
@PROV-139
@rust
@provider-settings
@tui
Feature: OpenAI profile streaming flag schema
  """
  The per-profile streaming flag lives as a flat Option<bool> on the wire type
  ProfileDefinition (codelet/rpc-types), mirroring how the compaction-threshold
  override is carried as flat optional fields. A canonical streaming_enabled()
  helper returns self.streaming.unwrap_or(true) so an absent flag means
  streaming is enabled. Verified by codelet/rpc-types/tests/prov139_streaming_flag.rs.
  """

  Background: User Story
    As a fspec user configuring an OpenAI-compatible endpoint
    I want the profile schema to carry a streaming flag that defaults to enabled
    So that streaming stays on by default while remaining explicitly toggleable

  Scenario: Absent streaming flag is treated as enabled
    Given a profile definition whose streaming flag is not set
    When the streaming_enabled helper is evaluated
    Then it reports streaming as enabled

  Scenario: An explicit streaming flag is echoed by the helper
    Given a profile definition whose streaming flag is set to disabled
    When the streaming_enabled helper is evaluated
    Then it reports streaming as disabled
