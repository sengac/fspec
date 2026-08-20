@PROV-142
@wip
@rust
@rpc
Feature: Per-profile autoContinue wire schema

  """
  The wire-portable ProfileDefinition (rust/rpc-types/src/lib.rs) carries a
  single flat Option<u32> field `auto_continue` (camelCase `autoContinue` on
  the wire) so the napi(object) projection stays a plain struct. None (key
  absent) or Some(0) mean OFF; Some(n) with n >= 1 means ON with budget n.
  The canonical predicate ProfileDefinition::auto_continue_enabled() mirrors
  streaming_enabled(). Research: spec/attachments/PROV-142/PROV-142-research.md.
  """

  Background: User Story
    As a developer using a local OpenAI-compatible server profile
    I want the profile's auto-continue default to round-trip through the wire shape
    So that the TUI form and the session seeder share one canonical field

  Scenario: The wire autoContinue predicate reports enabled only for positive budgets
    Given a wire profile definition whose autoContinue value is 300
    When the auto-continue enabled predicate is evaluated
    Then it reports enabled
    And a wire profile definition whose autoContinue value is 0 reports disabled
    And a wire profile definition with no autoContinue value reports disabled

  Scenario: The wire autoContinue value round-trips through JSON
    Given a wire profile definition whose autoContinue value is 300
    When the definition is serialized to JSON and back
    Then the autoContinue key carries 300
    And a legacy config file without the autoContinue key deserializes as absent
