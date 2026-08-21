@UPD-002 @cli @update @high
Feature: In-place self-update via fspec update CLI subcommand

  """
  The `fspec update` CLI subcommand. It calls the SAME shared
  codelet-fspec-core::update engine as the TUI `/update` command (rule [0]:
  one engine, no duplication). Headless output so the update path works
  without the TUI. `--check` is scriptable: exit 0 when current, exit 1 when
  a newer release is available.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. `fspec update` and the TUI /update MUST share one update engine —
  #      no duplicated download/replace logic
  #   2. `fspec update --check` MUST print the latest available version and
  #      exit 1 when a newer release is available, exit 0 when current
  #   3. `fspec update` (without --check) downloads + verifies + replaces
  #      the installed binary and prints a human-readable result
  #
  # EXAMPLES:
  #   1. `fspec update --check` on an outdated install prints the latest
  #      version and exits 1
  #   2. `fspec update --check` on the latest install exits 0
  #   3. `fspec update` on an outdated install installs the new binary and
  #      prints a success line naming the new version
  #
  # ========================================

  Background: User Story
    As a fspec user
    I want to run `fspec update` to upgrade fspec in place from the CLI
    So that the update path works without the TUI and is scriptable

  Scenario: fspec update --check reports availability via exit code
    Given fspec is installed at an older version
    When the user runs `fspec update --check`
    Then it prints the latest available version
    And it exits with code 1
    And when fspec is installed at the latest version, `fspec update --check` exits with code 0

  Scenario: fspec update installs the latest release in place
    Given fspec is installed at an older version
    And a newer release exists with an asset for the current platform
    When the user runs `fspec update`
    Then it installs the new binary
    And it prints a success line naming the new version

  Scenario: /update and fspec update share one update engine
    Given the /update TUI command and the `fspec update` CLI subcommand are both implemented
    When both are exercised against the same release
    Then both use the same shared download-verify-replace engine
    And no download or replacement logic is duplicated between them
