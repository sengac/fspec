@done
@workflow-automation
@cli
@RPC-326
Feature: workflow-automation CLI subcommand (Rust binary)
  """
  Front door #1 (shell argv): rust/fspec/src/workflow_automation.rs is the thin clap bridge for the
  `workflow-automation <action> <work-unit-id>` subcommand with --event / --from-state flags. It marshals
  the positional action + work-unit-id and the two optional flags into the JSON args shape and delegates to
  fspec_core::commands::workflow_automation::run. On success it prints nothing (parity with the TS shell,
  whose sub-functions return void) and exits 0; on a core error it writes the message to stderr and exits 1.
  --help is intercepted before clap and must be byte-for-byte identical to
  tests/fixtures/help/workflow-automation.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES (CLI surface):
  #   1. `fspec workflow-automation record-iteration <id>` succeeds (exit 0) on an existing unit, incrementing metrics.iterations.
  #   2. `fspec workflow-automation auto-advance <id> --event tests-pass --from-state testing` advances a testing unit, exit 0.
  #   3. `fspec workflow-automation validate-alignment <id>` exits 0 and does not modify work-units.json.
  #   4. A non-existent work unit exits 1 with the "Work unit '<id>' does not exist" error on stderr.
  #   5. `fspec workflow-automation --help` exits 0 and matches the TS formatCommandHelp fixture byte-for-byte.
  #   6. The CLI bridge contains NO action-dispatch / transition / mutation logic — it only marshals args and delegates.
  #
  # ========================================
  Background: User Story
    Given the fspec Rust binary exposes workflow-automation as a clap subcommand
    And the bridge rust/fspec/src/workflow_automation.rs delegates to fspec_core::commands::workflow_automation::run

  Scenario: Shell record-iteration increments the counter and exits 0
    Given a working directory whose spec/work-units.json contains AUTH-001
    When I run `fspec workflow-automation record-iteration AUTH-001` from that directory
    Then the command exits with code 0
    And the persisted AUTH-001 has metrics.iterations equal to 1

  Scenario: Shell auto-advance advances a testing unit and exits 0
    Given a working directory whose spec/work-units.json contains AUTH-001 with status 'testing'
    When I run `fspec workflow-automation auto-advance AUTH-001 --event tests-pass --from-state testing` from that directory
    Then the command exits with code 0
    And the persisted AUTH-001 status is 'implementing'

  Scenario: Shell command on a missing work unit exits 1 with an error
    Given a working directory whose spec/work-units.json contains no work unit MISSING-001
    When I run `fspec workflow-automation record-iteration MISSING-001` from that directory
    Then the command exits with code 1
    And stderr contains "Work unit 'MISSING-001' does not exist"

  Scenario: workflow-automation --help is byte-for-byte identical to the TS reference
    Given the fspec Rust binary has been compiled
    When I run `fspec workflow-automation --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/workflow-automation.txt

  Scenario: CLI bridge delegates to the same fspec_core function as the dispatcher
    Given the CLI bridge module rust/fspec/src/workflow_automation.rs
    When I inspect its source
    Then it contains no inline action-dispatch, transition, or work-units mutation logic
    And its only computation is JSON arg marshalling before delegating to fspec_core::commands::workflow_automation::run
