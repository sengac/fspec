@done
@cli
@installer
@cross-platform
@BUG-555
Feature: Installing fspec while fspec is running
  """
  On Linux, cp onto a running executable fails with "Text file busy", so scripts/build-install.sh and scripts/install.sh could not install while the TUI or daemon ran. Both scripts now copy the binary to a temporary file in the install directory (same filesystem), chmod it, and mv it over the target: rename replaces the directory entry, and running processes keep the old inode. On failure the temporary file is removed and the old binary stays. The update engine (fspec-core update/replace.rs, UPD-002) already extracts to a temporary file in the install directory and renames it over the binary on unix. The Windows scripts (build-install.ps1, install.ps1) are unchanged (see the assumption). Tests: rust/fspec/tests/bug555_atomic_binary_install.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. scripts/build-install.sh and scripts/install.sh install the binary atomically: copy it to a temporary file in the install directory, make it executable, then rename it over the target; they never write into the existing fspec file
  #   2. Installing over a running fspec succeeds; running fspec processes keep the old version (they hold the old file) and the installer says so: running fspec processes keep the old version until restarted
  #   3. When the install fails, the temporary file is removed and the existing fspec is left untouched
  #   4. The update engine (fspec update, UPD-002) already renames a temporary file in the install directory over the binary on unix; it replaces a running fspec and its output says to restart fspec
  #
  # EXAMPLES:
  #   1. A developer runs scripts/build-install.sh while the TUI runs ~/.local/bin/fspec: the build installs, the running TUI keeps working, ~/.local/bin/fspec is the new build and the script prints that running fspec processes keep the old version until restarted
  #   2. scripts/install.sh installs a release while fspec is running from the install directory: the install succeeds and the new binary is in place
  #   3. The update engine replaces an install path whose binary is running: the rename succeeds and the install path holds the new binary
  #   4. scripts/build-install.sh cannot write to a read-only install directory: it exits non-zero, the existing fspec is unchanged and no temporary file is left behind
  #
  # QUESTIONS (ANSWERED):
  #   Q: build-install.ps1 and install.ps1 use Copy-Item over fspec.exe, which also fails while fspec.exe runs on Windows. Should they move the running exe aside (fspec.exe.old) before copying? Conservative choice taken: left unchanged in BUG-555 (cannot be tested here); needs a Windows card.
  #   A: Not decided by the product owner yet; conservative choice taken by the helper (2026-11-06): the Windows scripts stay unchanged in BUG-555 because they cannot be exercised here. The supervisor is told so a Windows card can be raised.
  #
  # ASSUMPTIONS:
  #   1. Not decided by the product owner yet; conservative choice taken by the helper (2026-11-06): the Windows scripts stay unchanged in BUG-555 because they cannot be exercised here. The supervisor is told so a Windows card can be raised.
  #
  # ========================================
  Background: User Story
    As a developer reinstalling fspec
    I want to install or update the fspec binary while fspec is running
    So that the install succeeds instead of failing with Text file busy after a full build

  Scenario: build-install.sh installs over a running fspec
    Given fspec is running from the install directory
    When the developer runs scripts/build-install.sh
    Then the script exits 0 and the install directory holds the new build
    And the running fspec process keeps running
    And the script says running fspec processes keep the old version until restarted

  Scenario: install.sh installs a release over a running fspec
    Given fspec is running from the install directory
    When the user runs scripts/install.sh
    Then the script exits 0 and the install directory holds the release binary
    And the script says running fspec processes keep the old version until restarted

  Scenario: The update engine replaces a running fspec
    Given the binary at the install path is running
    When the update engine replaces the install path with a new binary
    Then the replace succeeds and the install path holds the new binary

  Scenario: A failed install leaves the existing fspec untouched
    Given the install directory holds an fspec binary and is read-only
    When the developer runs scripts/build-install.sh
    Then the script exits non-zero
    And the existing fspec is unchanged and no temporary file is left in the install directory
