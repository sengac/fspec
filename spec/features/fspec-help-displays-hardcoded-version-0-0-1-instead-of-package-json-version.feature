@done
@help-system
@cli
@phase1
@BUG-021
Feature: fspec --help displays hardcoded version 0.0.1 instead of package.json version
  """
  Version string is read from package.json at build time using TypeScript/Vite. Build process embeds version into dist bundle. If version cannot be read, the version line is omitted from --help output.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Version displayed in --help must match package.json version exactly
  #   2. Version should update automatically when package.json changes (no manual hardcoding)
  #   3. Build time - version embedded in dist bundle during build (typical for CLI tools)
  #   4. Don't display version string if unavailable
  #
  # EXAMPLES:
  #   1. When package.json has version 0.2.1, running 'fspec --help' displays 'Version 0.2.1'
  #   2. After bumping package.json to version 0.3.0 and rebuilding, 'fspec --help' displays 'Version 0.3.0'
  #   3. Currently shows 'Version 0.0.1' regardless of package.json value (bug to fix)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should version be read at build time (embedded in dist bundle) or runtime (reading package.json when --help runs)? Build-time is typical for CLI tools.
  #   A: true
  #
  #   Q: If we can't read the version (edge case), should we show 'unknown' or fallback to a default value?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to run fspec --help to check version
    So that I see the current version from package.json, not a hardcoded value

  Scenario: Display current version from package.json
    Given package.json has version "0.2.1"
    And the project has been built with npm run build
    When I run "fspec --help"
    Then the output should contain "Version 0.2.1"

  Scenario: Display updated version after package.json change
    Given package.json has version "0.3.0"
    And the project has been rebuilt with npm run build
    When I run "fspec --help"
    Then the output should contain "Version 0.3.0"

  Scenario: Omit version line when version cannot be read
    Given the version cannot be read from package.json
    When I run "fspec --help"
    Then the output should not contain a "Version" line
