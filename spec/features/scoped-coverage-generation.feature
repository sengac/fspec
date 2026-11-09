@REFAC-020
@integration
@coverage-tracking
@code-quality
@phase-1
Feature: Scoped lossless coverage generation
  """
  Both front doors call the existing core command run entrypoints. Indexed
  implementation linking and unlinking use an exact test-file identity and a
  canonical scenario hash checked inside a stable cross-process transaction.
  Canonical hashing sorts object keys but preserves every ordered array and
  unknown member. Explicit mappingJson readback leaves legacy JSON unchanged.
  Selected generation plans against raw JSON before applying any changes.
  Only link, unlink and global or selected generation participate in this
  bounded transaction protocol. Scenario rename/delete/create commands, report
  writes and the old running daemon are nonparticipants. The one-time pinned
  migration must exclude concurrent nonparticipant writes. This feature makes
  no repository-wide transaction claim. All maintained touched sources remain
  below 300 lines. No global generation bootstraps these acceptance sidecars;
  parent-reviewed scoped migration after GREEN and review will create them.
  """

  Background: User Story
    As a maintainer preserving scenario coverage lineage
    I want exact fresh-state mutations and lossless selected generation
    So that duplicate lineage and unrelated metadata survive relocation

  Scenario: Selected generation does not touch unrelated sidecars or missing files
    Given one changed feature and unrelated drifted or missing sidecars
    When only the changed feature is generated
    Then only its sidecar gains the newly declared scenario

  Scenario: Selected generation preserves unknown fields order and duplicate lineage
    Given selected metadata with unknown fields ordered duplicates and outlines
    When the selected feature gains newly declared scenarios
    Then retained metadata remains lossless and new scenario names follow feature order

  Scenario: Mapped scenario removal blocks all selected changes
    Given a selected feature with new scenarios and complete or incomplete obsolete mapped lineage
    When generation is previewed and applied
    Then the entire plan is blocked without partial additions or lineage removal

  Scenario: Ambiguous same name scenarios block selected generation without writes
    Given selected metadata with duplicate scenario names and a newly declared scenario
    When selected generation is previewed and applied to the ambiguous metadata
    Then both operations block without merging duplicate scenarios or adding partial metadata

  Scenario: Invalid or unreadable existing metadata is never recreated
    Given invalid malformed or unreadable selected metadata
    When generation is previewed and applied
    Then the blocked metadata is never recreated and unrelated bytes remain unchanged

  Scenario: Dry run and apply report the same actual selected plan
    Given an unchanged selected-generation input with additions and empty removals
    When preview is followed by apply and a no-op apply
    Then preview and apply report the same deltas and preview and no-op preserve bytes

  Scenario: Invalid ambiguous or missing feature never falls back to global generation
    Given unrelated features and ambiguous nested feature basenames
    When invalid ambiguous or absent selected names are requested
    Then generation fails without global fallback or unrelated writes
