@done
@COV-078
@cli
@coverage-tracking
@rust
Feature: Audit fix relinks a scenario to its exact step block
  """
  A test mapping stores raw line numbers, so any edit above a test moves
  its @step comments out of the mapped lines. fspec audit --fix repairs that
  mechanically from exact facts only: a scenario's exact @step block in a
  file is a line range whose @step comments, read in line order, are exactly
  the scenario's steps in order (other lines may sit between them, other @step
  comments may not). --fix looks for the block in the scenario's mapped test
  files, then in files the working tree adds (untracked, or staged as added)
  that no other feature maps. Found exactly once, the mappings of the old file
  become <file>:<first @step line>-<last @step line> through link-coverage's
  replace operation (BUG-502), keeping their implementation mappings, inside
  the coverage sidecar transaction. Found zero or several times, nothing is
  written and the problem names the count and the candidate ranges. --fix also
  removes coverage entries whose scenario is not in the feature and deletes
  coverage files without their feature. link-coverage --test-file without
  --test-lines links the exact block found once. Only comment text is read, so
  it works for any language.
  Core: rust/fspec-core/src/commands/audit/relink.rs and
  rust/fspec-core/src/commands/link_coverage/step.rs. Tests:
  rust/fspec-core/tests/cov078_audit_fix_relinks_exact_step_blocks.rs.
  """

  Background: User Story
    As a maintainer whose tests move as code changes
    I want to have fspec audit --fix relink a scenario to its exact @step block when that block is found once
    So that drifted mappings are repaired mechanically instead of by hand, with nothing guessed

  Scenario: A drifted mapping is relinked to the block found once in its file
    Given scenario "Log in" maps tests/login_test.rs:10-14 with an implementation mapping of src/login.rs
    And its @step comments moved to lines 22-26
    When fspec audit --fix runs
    Then the mapping becomes tests/login_test.rs:22-26 and keeps its src/login.rs implementation mapping
    And it prints "tests/login_test.rs:10-14 -> tests/login_test.rs:22-26"
    And a second fspec audit --fix changes nothing

  Scenario: A relinkable drift names the fix command
    Given scenario "Log in" maps tests/login_test.rs:10-14 but its @step comments moved to lines 22-26
    When fspec audit --json runs
    Then the drifted problem has fix kind "mechanical" and command "fspec audit --fix --feature login"

  Scenario: A test moved into a new untracked file is relinked to that file
    Given a git repository where scenario "Log in" maps tests/login_test.rs:10-14
    And the test moved into the new untracked file tests/login_api_test.rs at lines 3-7
    When fspec audit --fix runs
    Then the mapping becomes tests/login_api_test.rs:3-7

  Scenario: A new file another feature maps is not searched
    Given a git repository where scenario "Log in" maps tests/login_test.rs:10-14 and its steps are gone from that file
    And the new untracked file tests/shared_test.rs holds the block but feature "admin" maps it
    When fspec audit --fix runs
    Then the mapping stays tests/login_test.rs:10-14
    And the problem says the exact @step block occurs 0 times

  Scenario: A block that occurs twice is not relinked
    Given scenario "Log in" maps tests/login_test.rs:1-5, which holds none of its steps
    And its exact @step block occurs at lines 10-14 and 40-44
    When fspec audit --fix runs
    Then the coverage file is unchanged
    And the problem says the exact @step block occurs 2 times (tests/login_test.rs:10-14, tests/login_test.rs:40-44)

  Scenario: Steps out of order are not an exact block
    Given scenario "Log in" maps tests/login_test.rs:1-5, which holds none of its steps
    And the file holds its @step comments only in another order
    When fspec audit --fix runs
    Then the coverage file is unchanged
    And the problem says the exact @step block occurs 0 times

  Scenario: A coverage entry for a scenario the feature no longer has is removed
    Given the coverage file lists scenario "Old" that the feature no longer has
    When fspec audit --fix runs
    Then the "Old" entry is removed
    And it prints the removal

  Scenario: A coverage file without its feature is deleted
    Given spec/features/old.feature.coverage exists without old.feature
    When fspec audit --fix runs
    Then spec/features/old.feature.coverage is deleted

  Scenario: link-coverage without test lines links the block found once
    Given the "Log in" @step block occurs once in tests/login_test.rs, at lines 22-26
    When fspec link-coverage login --scenario "Log in" --test-file tests/login_test.rs runs without --test-lines
    Then the scenario maps tests/login_test.rs:22-26

  Scenario: link-coverage without test lines refuses an ambiguous block
    Given the "Log in" @step block occurs twice in tests/login_test.rs
    When fspec link-coverage login --scenario "Log in" --test-file tests/login_test.rs runs without --test-lines
    Then it fails saying the exact @step block occurs 2 times
    And the coverage file is unchanged
