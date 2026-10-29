@REFAC-020
@integration
@coverage-tracking
@code-quality
@phase-1
Feature: Precise lossless coverage mutations
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

  Scenario: Readback exposes ordered rows and canonical scenario hash
    Given duplicate mapping rows with unknown metadata
    When explicit machine readback is requested
    Then row indices and the complete canonical scenario hash are exposed without writes

  Scenario: Selected second duplicate gains complete two file union
    Given two same-file mapping rows with distinct retained lineage
    When the second row gains another physical implementation using its fresh hash
    Then only the selected row gains the complete two file union

  Scenario: Selected obsolete duplicate is removed without losing other occurrences
    Given identical duplicate rows and retained implementation arrays
    When one obsolete occurrence is explicitly unlinked with its fresh hash
    Then every other mapping occurrence and unknown member is retained

  Scenario: Invalid or stale selector leaves exact sidecar bytes unchanged
    Given a valid duplicate-row coverage sidecar
    When stale malformed mismatched unpaired or incompatible selectors are submitted
    Then every rejected request leaves exact sidecar bytes unchanged

  Scenario: Ambiguous same name scenarios reject indexed mutations without writes
    Given two raw coverage scenarios with the same name and distinct metadata
    When indexed linking and unlinking select that ambiguous name
    Then neither scenario is chosen and exact sidecar bytes remain unchanged

  Scenario: Selected link retains file and story step validation
    Given a fresh selector for a story coverage row
    When missing files or tests without matching step comments are linked
    Then normal validation rejects the request without writes

  Scenario: Unselected mutation retains legacy duplicate selection behavior
    Given two same-file mapping rows with distinct retained lineage
    When implementation linking and unlinking omit the new selector
    Then the legacy first matching row behavior is unchanged

  Scenario: Selected generation does not touch unrelated sidecars or missing files
    Given one changed feature and unrelated drifted or missing sidecars
    When only the changed feature is generated
    Then only its sidecar gains the newly declared scenario

  Scenario: Selected generation preserves unknown fields order and duplicate lineage
    Given selected metadata with unknown fields ordered duplicates outlines and rule scenarios
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

  Scenario: Competing selected mutations allow one fresh hash and reject the stale one
    Given two processes selecting the same scenario revision
    And an externally held transaction lock for equivalent feature spellings
    When both attempt distinct implementation additions
    Then both remain blocked without changing sidecar bytes while the lock is held
    And exactly one commits and the other rejects its stale hash without lost updates after release

  Scenario: Legacy link and unlink wait for the shared sidecar transaction
    Given an externally held stable sidecar transaction lock
    When legacy link and unlink run in separate processes using equivalent feature spellings
    Then each waits and reads the latest committed mappings after lock release

  Scenario: Selected and global generation wait then preserve latest participating write
    Given an externally held stable sidecar transaction lock
    When selected and global generation run in separate processes using equivalent feature spellings
    Then each waits and preserves the latest mappings after lock release
