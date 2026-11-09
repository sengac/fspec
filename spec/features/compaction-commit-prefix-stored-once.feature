@CMPCT-086
@persistence
@context-management
Feature: A compaction commit stores its covered prefix once
  """
  CommittedGeneration (rust/core/src/persistence/compaction_boundary.rs) carried covered_prefix, an exact copy of
  source.prefix that validate_committed_boundary requires to be equal, so every compacted manifest stored the prefix
  UUIDs twice (21% of manifest bytes in the CMPCT-084 sample). covered_prefix becomes CommittedGeneration::covered_prefix()
  returning &source.prefix; a legacy `covered_prefix` key is still accepted on load (serde default, never serialised)
  and must still equal source.prefix. checkpoint.payload, source.prefix, verified_suffix and identity stay because
  restore and the lineage checks read them. Research: spec/attachments/CMPCT-086/research-commit-bookkeeping.md.
  """

  Background: User Story
    As an fspec user with long compacted sessions
    I want compaction commits to store the covered prefix once
    So that manifests are smaller and rewrites cheaper without weakening restore checks

  Scenario: A committed generation serialises its prefix once
    Given a committed generation over a three-message prefix
    When it is serialised into a manifest
    Then the prefix message ids appear once
    And no covered_prefix key is written

  Scenario: A legacy manifest with a matching covered prefix still loads and validates
    Given a manifest written with a covered_prefix equal to source.prefix
    When it is loaded and its committed boundary is validated
    Then validation succeeds
    And the covered prefix is the source prefix

  Scenario: A legacy manifest whose covered prefix differs is still rejected
    Given a manifest written with a covered_prefix that differs from source.prefix
    When its committed boundary is validated
    Then validation fails as an incompatible committed generation boundary
