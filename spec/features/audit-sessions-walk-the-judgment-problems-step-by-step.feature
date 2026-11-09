@BUG-582
@done
@AUDIT-004
@cli
@coverage-tracking
@reverse-acdd
@rust
Feature: Audit sessions walk the judgment problems step by step
  """
  fspec audit reports exact facts only. A problem that needs judgment (its
  command holds a <placeholder>, or its exact @step block was not found exactly
  once) has fix kind guided and names 'fspec audit --start --feature <f>'.
  fspec audit --start [--feature <f>]... [--work-unit <id>] [--area <a>]...
  creates a session, like fspec reverse, whose steps are the scope's guided
  problems in report order; an explicit --area meaning adds one review step per
  live scenario and an explicit --area spec one stale-text review step per
  feature. --status, --continue, --reset and --complete take the same scope.
  --continue on a problem step re-runs the audit for that step's feature and
  area and stays while a problem with the same area, code, feature, scenario
  and location remains; on a review step it records the attestation. --complete
  deletes the session only when every step was continued. fspec runs no tests
  and decides nothing that needs judgment: the meaning step prints the mapped
  lines, the configured tools.test.command and the scratch-worktree procedure.
  Sessions live in the temp dir, keyed by command, project root and scope
  (types/guided_session.rs); fspec reverse uses the same module and keeps its
  session file. Core: rust/fspec-core/src/commands/audit.rs and
  rust/fspec-core/src/commands/audit/session.rs. Tests:
  rust/fspec-core/tests/audit004_audit_sessions_walk_judgment_problems.rs.
  """

  Background: User Story
    As a AI agent auditing an fspec project
    I want to walk the audit problems that need judgment step by step in a session, with exact facts and the fspec commands to use at each step
    So that I resolve drift that fspec cannot repair mechanically without fspec guessing anything

  Scenario: A problem that needs judgment names the audit session as its command
    Given the @step block of "Log in" occurs twice in tests/login_test.rs and its mapping of tests/login_test.rs:10-20 drifted
    When fspec audit --feature login runs
    Then the "coverage drifted login" line ends in "-> fspec audit --start --feature login"
    And the problem's fix kind in the JSON report is "guided"

  Scenario: Starting a session over a twice-found block creates one step naming both candidates
    Given the @step block of "Log in" occurs twice in tests/login_test.rs and its mapping of tests/login_test.rs:10-20 drifted
    When fspec audit --start --feature login runs
    Then a session with 1 step is created
    And step 1 names both candidate ranges and the link-coverage command with --replace

  Scenario: Continuing while the problem persists stays on the step
    Given an audit session for feature "login" on step 1 of 1 whose mapping still drifts
    When fspec audit --continue --feature login runs
    Then it says the problem persists and exits 1
    And fspec audit --status --feature login still shows step 1 of 1

  Scenario: Continuing after the repair advances and completing deletes the session
    Given an audit session for feature "login" on step 1 of 1
    And the mapping was relinked to the first candidate range
    When fspec audit --continue --feature login runs
    Then the session has every step done
    And fspec audit --complete --feature login deletes the session

  Scenario: Completing before every step is done is refused
    Given an audit session for feature "login" on step 1 of 1 whose mapping still drifts
    When fspec audit --complete --feature login runs
    Then it fails saying not every step is done
    And the session still exists

  Scenario: Resetting deletes the session
    Given an audit session for feature "login" on step 1 of 1
    When fspec audit --reset --feature login runs
    Then the session no longer exists
    And fspec audit --status --feature login says there is no audit session for the scope

  Scenario: A scope without guided problems creates no session
    Given feature "login" audits clean
    When fspec audit --start --feature login runs
    Then it says there is nothing to walk and no session is created

  Scenario: The meaning area adds one review step per scenario with its mappings and the test command
    Given work unit AUTH-001 tags feature "login" whose two live scenarios map tests and implementation lines
    And spec/fspec-config.json configures tools.test.command "cargo test"
    When fspec audit --start --area meaning --work-unit AUTH-001 runs
    Then a session with 2 review steps is created
    And step 1 prints the scenario's test mapping, its implementation mappings and "cargo test"
    And step 1 tells the agent to break the implementation lines in a scratch git worktree, run the test, confirm it fails for the missing behaviour and remove the worktree

  Scenario: Continuing a review step records the attestation
    Given an audit session over the meaning area of AUTH-001 on step 1 of 2
    When fspec audit --continue --area meaning --work-unit AUTH-001 runs
    Then the session is on step 2 of 2
    And step 1 records when it was attested

  Scenario: The spec area adds one stale-text review step per feature
    Given features "login" and "admin" audit clean
    When fspec audit --start --area spec --feature login --feature admin runs
    Then a session with 2 review steps is created, one per feature, naming the feature file and its implementation files

  Scenario: Sessions of different scopes are separate
    Given features "login" and "admin" each have a twice-found drifted @step block
    When fspec audit --start --feature login and fspec audit --start --feature admin run in one project
    Then each scope has its own session file
    And fspec audit --status --feature admin shows the admin step

  Scenario: The meaning area without start is an error
    When fspec audit --area meaning --feature login runs
    Then it fails saying the meaning area is accepted only with --start

  Scenario: The validating reminder offers the meaning session
    Given work unit AUTH-001 moves from implementing to validating
    When the move succeeds
    Then its reminder offers "fspec audit --start --area meaning --work-unit AUTH-001"

  Scenario: fspec reverse keeps its session file on the shared session module
    Given a project with no reverse session
    When fspec reverse starts a session
    Then the session file is fspec-reverse-<first 12 hex of sha256 of the canonical project root>.json in the temp directory
