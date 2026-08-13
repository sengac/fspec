@done
@workflow-automation
@cli
@RPC-198
Feature: auto-advance CLI subcommand (Rust binary)
  """
  Front door #1 (shell argv): rust/fspec/src/auto_advance.rs is the thin clap bridge for the
  `auto-advance` subcommand. Per Framing A the TS Commander shell is broken — it calls autoAdvance({dryRun})
  without a workUnitId — so the Rust bridge reproduces that: it marshals an empty args object (ignoring
  --dry-run) and the core surfaces 'Work unit undefined not found', wrapped as
  'Failed to auto-advance: Work unit undefined not found', exit 1. --help is intercepted before clap and
  must be byte-for-byte identical to tests/fixtures/help/auto-advance.txt.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES (CLI surface):
  #   1. `fspec auto-advance` reproduces the broken Framing-A shell: exits 1, prints
  #      '✗ Failed to auto-advance: Work unit undefined not found' to stderr.
  #   2. `fspec auto-advance --dry-run` behaves identically (the flag is accepted but ignored).
  #   3. `fspec auto-advance --help` exits 0 and matches the TS formatCommandHelp fixture byte-for-byte.
  #   4. The CLI bridge contains NO transition/state-mutation logic — it only marshals args and delegates.
  #
  # ========================================
  Background: User Story
    Given the fspec Rust binary exposes auto-advance as a clap subcommand
    And the bridge rust/fspec/src/auto_advance.rs delegates to fspec_core::commands::auto_advance::run

  Scenario: Shell auto-advance reproduces the broken Framing-A failure
    Given a working directory with a valid spec/work-units.json
    When I run `fspec auto-advance` from that directory
    Then the command exits with code 1
    And stderr contains '✗ Failed to auto-advance:'
    And stderr contains 'Work unit undefined not found'

  Scenario: Shell auto-advance with --dry-run behaves identically
    Given a working directory with a valid spec/work-units.json
    When I run `fspec auto-advance --dry-run` from that directory
    Then the command exits with code 1
    And stderr contains 'Work unit undefined not found'

  Scenario: auto-advance --help is byte-for-byte identical to the TS reference
    Given the fspec Rust binary has been compiled
    When I run `fspec auto-advance --help` piped to non-TTY
    Then the command exits 0
    And stdout is byte-for-byte identical to the fixture at rust/fspec/tests/fixtures/help/auto-advance.txt

  Scenario: CLI bridge delegates to the same fspec_core function as the dispatcher
    Given the CLI bridge module rust/fspec/src/auto_advance.rs
    When I inspect its source
    Then it contains no inline state-transition or work-units mutation logic
    And its only computation is JSON arg marshalling before delegating to fspec_core::commands::auto_advance::run
