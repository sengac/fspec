@UPD-002
@cli
@update
@cross-platform
@high
Feature: In-place self-update engine
  """
  The shared download-verify-replace engine lives in codelet-fspec-core::update.
  It is the single source of truth for both the `fspec update` CLI subcommand
  and the TUI `/update` slash command (rule [0]: one engine, no duplication).

  Manual reqwest+sha2 download path (NOT the self_update crate) so the engine
  can be pointed at a local mock GitHub API via a base_url override — immune
  to crate version drift. self-replace is used only for the Windows
  locked-.exe rename. Full plan in attachment self-update-implementation.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The downloaded binary MUST be verified against the SHA-256 digest
  #      GitHub publishes for the release asset BEFORE it replaces the
  #      installed binary
  #   2. If the running binary is already the latest release, the engine MUST
  #      report up-to-date and make no changes
  #   3. The new binary MUST be written to a temp file and only renamed into
  #      place after the checksum passes — a failed download or checksum
  #      mismatch MUST leave the installed binary untouched
  #   4. Latest only in v1. The engine always targets the newest release;
  #      specific-version install (--version) is a follow-up.
  #
  # EXAMPLES:
  #   1. Engine on v0.10.0 with v0.10.0 the latest release: reports
  #      up-to-date, installed binary untouched
  #   2. Engine on v0.9.3 with v0.10.0 the latest release: downloads the
  #      current-platform asset, verifies its SHA-256, replaces the installed
  #      binary, reports the new version
  #   3. Engine with no network: returns a Network error, installed binary
  #      untouched
  #   4. Engine where the published digest does not match the asset: returns
  #      a ChecksumMismatch error, installed binary untouched
  #
  # ========================================
  Background: User Story
    As a fspec user
    I want the update engine to safely upgrade fspec in place
    So that both the TUI and CLI share one verified download-verify-replace path

  Scenario: Engine reports up-to-date when already on the latest release
    Given the engine is configured at the latest released version
    When the engine checks for the latest release
    Then it reports up-to-date
    And the installed binary is unchanged

  Scenario: Engine installs the latest release in place
    Given the engine is configured at an older version
    And a newer release exists with an asset for the current platform
    When the engine performs an update
    Then it reports the new version
    And the installed binary is replaced with the downloaded binary

  Scenario: Engine fails safely with no network
    Given the engine is configured at an older version
    And the network is unreachable
    When the engine performs an update
    Then it returns a network error
    And the installed binary is unchanged

  Scenario: Engine verifies the checksum before replacing the binary
    Given a newer release exists with an asset for the current platform
    And the published SHA-256 digest does not match the asset content
    When the engine performs an update
    Then it returns a checksum mismatch error
    And the installed binary is unchanged
