@done
@cli
@installer
@cross-platform
@BUG-564
Feature: Installing fspec on Windows while fspec is running
  """
  BUG-555 made the Linux and macOS scripts install atomically; scripts/build-install.ps1 and scripts/install.ps1 still ran Copy-Item over fspec.exe, which fails on Windows while fspec runs (a running .exe cannot be overwritten or deleted, only renamed). Both scripts now call one function, Install-FspecBinary, identical in both files: it removes the fspec.exe.*.old files no process holds, copies the new binary to .fspec-install-<id>.tmp in the install directory, renames fspec.exe to fspec.exe.<id>.old, renames the temporary file to fspec.exe, and prints that running fspec processes keep the old version until restarted. On failure it removes the temporary file and moves the old fspec.exe back. PowerShell and Windows are not available where this was built, so rust/fspec-core/tests/bug564_windows_installers_rename_aside.rs checks the scripts' text exactly; running them on Windows was not done.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Same rule as BUG-555 (10dcce812) for scripts/build-install.ps1 and scripts/install.ps1: never copy over fspec.exe in place
  #   2. Both scripts install through one function, Install-FspecBinary, whose text is identical in both files: copy the new binary to a temporary file in the install directory, rename the current fspec.exe aside to fspec.exe.<id>.old (Windows can rename a running .exe but not overwrite or delete it), then rename the temporary file to fspec.exe
  #   3. Each install first removes the fspec.exe.*.old files no running process holds; files still held stay until a later install (a unique <id> per install, so a second install while the first old binary still runs does not collide)
  #   4. On failure the temporary file is removed, a renamed-aside fspec.exe is moved back, and the error is reported (install.ps1 keeps its Administrator hint)
  #   5. After installing, the scripts say: Running fspec processes keep the old version until restarted.
  #   6. PowerShell is not available in this environment (no pwsh, no Windows), so the tests check the scripts' text exactly (the shared function, its order of operations, no remaining Copy-Item onto fspec.exe); running them on Windows is recorded as not done
  #
  # EXAMPLES:
  #   1. fspec.exe is running from the install directory; scripts/build-install.ps1 renames it to fspec.exe.<id>.old, moves the new build in, exits 0 and prints the running-processes message
  #   2. A previous install left fspec.exe.3f2a.old whose process has exited; the next install deletes it before installing
  #   3. Copying to the temporary file fails (read-only directory); fspec.exe is unchanged and no .fspec-install-*.tmp file is left
  #
  # ASSUMPTIONS:
  #   1. Renaming a running executable on the same volume is allowed on Windows (MoveFileEx without MOVEFILE_REPLACE_EXISTING on the image file); not verified here because no Windows host or pwsh is available
  #
  # ========================================
  Background: User Story
    As a developer or user installing fspec on Windows
    I want to install or update fspec.exe while fspec is running
    So that the installer succeeds instead of failing because the running fspec.exe is locked

  Scenario: Both Windows installers install through the same rename-aside function
    Given scripts/build-install.ps1 and scripts/install.ps1
    When their install steps are read
    Then each defines Install-FspecBinary with the same text
    And each installs fspec.exe only by calling Install-FspecBinary
    And neither copies onto fspec.exe in place

  Scenario: The new binary replaces fspec.exe by renames inside the install directory
    Given the Install-FspecBinary function
    When its steps are read in order
    Then it removes the fspec.exe.*.old files no process holds
    And it copies the new binary to a temporary file in the install directory
    And it renames fspec.exe aside to fspec.exe.<id>.old before renaming the temporary file to fspec.exe
    And it prints "Running fspec processes keep the old version until restarted."

  Scenario: A failed install leaves fspec.exe as it was
    Given the Install-FspecBinary function
    When copying or renaming fails
    Then it removes the temporary file
    And it moves a renamed-aside fspec.exe back
    And it rethrows the error
