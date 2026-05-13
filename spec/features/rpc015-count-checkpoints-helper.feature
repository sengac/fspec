@done
@RPC-015
@rust
@git
@checkpoint
@checkpoint-management
@git-ops
Feature: RPC-015 count_checkpoints helper — classify refs/fspec-checkpoints/

  """
  RPC-015 (slice 1a of 3) — Adds a new `codelet_git::ghost_commit::count_checkpoints`
  helper that iterates every git ref under `refs/fspec-checkpoints/` and
  classifies each by whether the last path segment (checkpoint name) contains
  the substring `-auto-` — names containing `-auto-` are counted as auto, all
  others as manual.

  Mirrors the TS rule from src/utils/checkpoint-index.ts:
    AUTO_CHECKPOINT_PATTERN = '-auto-'
    isAutomaticCheckpoint(name) === name.includes('-auto-')

  Test pair: codelet/git/tests/count_checkpoints_rpc015.rs.
  """

  Background: User Story
    As a Rust fspec TUI developer
    I want a single pure helper that aggregates checkpoint counts across all work units
    So that both the new tarpc `FspecService::checkpoint_counts` AND the additive `napi::count_checkpoints` NAPI export delegate to ONE source of truth

  Scenario: count_checkpoints returns zero in a directory that is not a git repo
    Given a temporary directory that has NOT been initialized as a git repository
    When codelet_git::ghost_commit::count_checkpoints is called with that directory
    Then the call succeeds and returns CheckpointCounts { manual: 0, auto: 0 }

  Scenario: count_checkpoints returns zero in a git repo with no checkpoint refs
    Given a temporary directory initialized as a git repository
    And no refs exist under refs/fspec-checkpoints/
    When codelet_git::ghost_commit::count_checkpoints is called against that repo
    Then the call returns CheckpointCounts { manual: 0, auto: 0 }

  Scenario: count_checkpoints classifies one manual and one auto checkpoint for a single work unit
    Given a temporary git repository
    And a ghost-checkpoint ref refs/fspec-checkpoints/AUTH-001/baseline exists (manual)
    And a ghost-checkpoint ref refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing exists (auto)
    When codelet_git::ghost_commit::count_checkpoints is called against that repo
    Then the result equals CheckpointCounts { manual: 1, auto: 1 }

  Scenario: count_checkpoints aggregates counts across multiple work units
    Given a temporary git repository
    And a ref refs/fspec-checkpoints/AUTH-001/baseline exists (manual)
    And a ref refs/fspec-checkpoints/AUTH-001/AUTH-001-auto-testing exists (auto)
    And a ref refs/fspec-checkpoints/BUG-002/BUG-002-auto-specifying exists (auto)
    When codelet_git::ghost_commit::count_checkpoints is called against that repo
    Then the result equals CheckpointCounts { manual: 1, auto: 2 }
