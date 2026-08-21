@UPD-002 @tui @slash-commands @update @high
Feature: In-place self-update via /update TUI command

  """
  The /update slash command in the TUI. Mirrors the /continue (CONT-002)
  slash-command pattern: update_parser.rs + slash_parser route +
  dispatch_slash_update.rs. The dispatch handler calls the shared
  codelet-fspec-core::update engine (rule [0]: one engine, no duplication).
  The TUI MUST NOT auto-restart or exec itself after an update (rule [6]).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /update MUST run non-interactively (no blocking stdin prompt)
  #      because the TUI terminal is in raw mode
  #   2. The TUI MUST NOT auto-restart or exec itself after an update —
  #      it reports the new version and instructs the user to restart
  #   3. On success the TUI shows a line naming the new version and
  #      instructing the user to restart fspec to activate it
  #   4. On error the TUI shows an error line and fspec keeps working at
  #      its current version
  #
  # EXAMPLES:
  #   1. User runs /update while already on the latest release: the TUI
  #      shows '✓ fspec is up to date' and nothing changes
  #   2. User runs /update on v0.9.3 while v0.10.0 is the latest release:
  #      the TUI shows a checking line, then '✓ fspec v0.10.0 installed.
  #      Restart fspec to activate.'
  #   3. User runs /update with no network: the TUI shows an error line and
  #      the installed binary is unchanged
  #
  # ========================================

  Background: User Story
    As a fspec user
    I want to run /update in the TUI to upgrade fspec in place
    So that I stay current without manual downloads or rebuilds

  Scenario: /update reports up-to-date when already on the latest release
    Given fspec is running at the latest released version
    When the user enters "/update"
    Then the TUI shows a message that fspec is up to date
    And the installed binary is unchanged

  Scenario: /update installs the latest release in place
    Given fspec is running at an older version
    And a newer release exists on GitHub with an asset for the current platform
    When the user enters "/update"
    Then the TUI shows a checking line while the release is looked up
    And the TUI shows a success line naming the new version and instructing the user to restart fspec

  Scenario: /update fails safely with no network
    Given fspec is running at an older version
    And the network is unreachable
    When the user enters "/update"
    Then the TUI shows an error line describing the failure
    And the installed binary is unchanged
    And fspec keeps working at its current version

  Scenario: /update never prompts for confirmation
    Given fspec is running in the TUI with the terminal in raw mode
    And a newer release exists on GitHub
    When the user enters "/update"
    Then the update proceeds without blocking on a stdin yes/no prompt

  Scenario: /update does not restart the running TUI
    Given fspec is running in the TUI
    And a newer release exists on GitHub
    When the user enters "/update" and the update succeeds
    Then the running TUI session continues without interruption
    And the new version activates on the next fspec launch
