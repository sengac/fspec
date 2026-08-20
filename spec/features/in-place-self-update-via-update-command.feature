@UPD-002 @tui @cli @slash-commands @update @cross-platform @high
Feature: In-place self-update via /update command

  """
  Use the self_update crate (jaemk, GitHub backend, checksums+async features) with .no_confirm(true). TUI wiring mirrors the /continue (CONT-002) slash-command pattern: update_parser.rs + slash_parser route + dispatch_slash_update.rs. Shared engine in codelet-fspec-core::update. Full plan in attachment self-update-implementation.md
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /update in the TUI and `fspec update` in the CLI MUST share one update engine — no duplicated download/replace logic
  #   2. The downloaded binary MUST be verified against the SHA-256 digest GitHub publishes for the release asset BEFORE it replaces the installed binary
  #   3. If the running binary is already the latest release, /update MUST report up-to-date and make no changes
  #   4. The update MUST run non-interactively (no_confirm, no blocking stdin prompt) because the TUI terminal is in raw mode and has no interactive stdin
  #   5. The new binary MUST be written to a temp file and only renamed into place after the checksum passes — a failed download or checksum mismatch MUST leave the installed binary untouched
  #   6. The TUI MUST NOT auto-restart or exec itself after an update — it reports the new version and instructs the user to restart fspec to activate it
  #   7. Latest only in v1. /update and `fspec update` always target the newest release; specific-version install (--version) is a follow-up.
  #
  # EXAMPLES:
  #   1. User runs /update while already on the latest release: the TUI shows '✓ fspec is up to date' and nothing changes
  #   2. User runs /update on v0.9.3 while v0.10.0 is the latest release: the TUI shows a checking line, then '✓ fspec v0.10.0 installed. Restart fspec to activate.' Quitting and relaunching fspec shows version 0.10.0
  #   3. User runs /update with no network: the TUI shows an error line and the installed binary is unchanged — fspec keeps working at its current version
  #   4. Running `fspec update --check` on an outdated install prints the latest available version and exits with code 1; on the latest install it exits with code 0
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should `fspec update` support installing a specific version (e.g. `fspec update --version 0.9.3` for downgrades/pinning), or only 'latest' in v1?
  #   A: Latest only in v1. /update and `fspec update` always target the newest release; specific-version install (--version) is a follow-up.
  #
  # ========================================

  Background: User Story
    As a fspec user
    I want to run /update to upgrade fspec in place
    So that stay current without manual downloads or rebuilds

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
    And after quitting and relaunching fspec, fspec --version reports the new version

  Scenario: /update fails safely with no network
    Given fspec is running at an older version
    And the network is unreachable
    When the user enters "/update"
    Then the TUI shows an error line describing the failure
    And the installed binary is unchanged
    And fspec keeps working at its current version

  Scenario: /update verifies the checksum before replacing the binary
    Given a newer release exists on GitHub with an asset for the current platform
    And the published SHA-256 digest for the asset does not match the asset content
    When the user enters "/update"
    Then the TUI shows a checksum error line
    And the installed binary is unchanged

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

  Scenario: fspec update --check reports availability via exit code
    Given fspec is installed at an older version
    When the user runs `fspec update --check`
    Then it prints the latest available version
    And it exits with code 1
    And when fspec is installed at the latest version, `fspec update --check` exits with code 0

  Scenario: /update and fspec update share one update engine
    Given the /update TUI command and the `fspec update` CLI subcommand are both implemented
    When both are exercised against the same release
    Then both use the same shared download-verify-replace engine
    And no download or replacement logic is duplicated between them
