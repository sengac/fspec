@done
@COV-077
@cli
@coverage-tracking
@rust
Feature: Commit gate runs fspec audit on the staged changes
  """
  Specs decayed because the audit was opt-in. fspec audit --changed limits
  the audit to the features a commit touches: features whose .feature or
  .feature.coverage file is staged, plus every feature mapping a staged test
  or implementation file (added, modified or deleted), found through the
  coverage sidecar index. Under --changed every file the audit reads comes
  from the git index, so another agent's unstaged edits neither block nor
  hide a problem in this commit, and a file staged as deleted is missing.
  fspec init and fspec --sync-version install and keep current a git
  pre-commit hook that runs only fspec audit --changed and blocks the commit
  unless it exits 0; it prints the audit's lines and never modifies files.
  There is no bypass flag and no warn-only mode. A pre-commit hook fspec did
  not write is never overwritten: fspec says the gate is not installed and
  prints the line to add. Product owner (2026-11-07): no CI step until
  COV-071..074 are finished.
  Core: rust/fspec-core/src/commands/audit/source.rs and
  rust/fspec-core/src/commands/init/commit_gate.rs. Tests:
  rust/fspec-core/tests/cov077_commit_gate_audits_staged_changes.rs.
  """

  Background: User Story
    As a maintainer of an fspec project
    I want to have every commit pass fspec audit --changed in a pre-commit hook
    So that coverage and spec drift is fixed when it is made instead of piling up

  Scenario: A commit that stages no spec, test or implementation file passes
    Given a git repository whose only staged file is README.md
    When fspec audit --changed --json runs
    Then it prints {"problems":[],"exitCode":0}

  Scenario: A staged edit that moves a scenario's steps out of its mapped lines is reported
    Given a git repository where scenario "Log in" maps tests/login_test.rs:1-3
    And an edit that moves its @step comments to lines 5-7 is staged
    When fspec audit --changed runs
    Then it reports "coverage drifted login" for tests/login_test.rs:1-3
    And it exits 1

  Scenario: An unstaged edit neither blocks nor hides a problem
    Given a git repository where scenario "Log in" maps tests/login_test.rs:1-3 and the staged test file is correct
    And an unstaged edit breaks the test file
    When fspec audit --changed runs
    Then it reports nothing for login
    And it exits 0

  Scenario: A staged scenario without a coverage entry is reported
    Given a git repository where spec/features/login.feature is staged with a new scenario "Log out" that has no coverage entry
    When fspec audit --changed runs
    Then it reports "coverage missing-entry login" naming "Log out"
    And it exits 1

  Scenario: A test file staged as deleted is missing
    Given a git repository where scenario "Log in" maps tests/login_test.rs:1-3
    And the deletion of tests/login_test.rs is staged while the file is still on disk
    When fspec audit --changed runs
    Then it reports "coverage missing-test-file login" for tests/login_test.rs:1-3

  Scenario: fspec init installs the pre-commit hook
    Given a git repository without a pre-commit hook
    When fspec init runs
    Then .git/hooks/pre-commit is an executable hook that runs only fspec audit --changed and exits with its exit code
    And the hook marks itself as written by fspec

  Scenario: A pre-commit hook fspec did not write is left alone
    Given a git repository whose .git/hooks/pre-commit was written by the user
    When fspec init runs
    Then the hook is unchanged
    And fspec prints that the commit gate is not installed and the line "fspec audit --changed || exit 1" to add to the hook

  Scenario: fspec --sync-version rewrites the hook fspec installed earlier
    Given a git repository whose pre-commit hook was written by an earlier fspec version
    When fspec --sync-version runs
    Then the hook is rewritten with the current fspec hook
