@BUG-582
@REFAC-020
@integration
@coverage-tracking
@code-quality
@phase-1
Feature: Coverage sidecar transactions
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
