@done
@high
@cli
@release-management
@automation
@CLI-009
Feature: Publish command for conditional npm publishing
  """
  Architecture notes:
  - Implemented as Claude Code slash command in .claude/commands/publish.md
  - Depends on /release command being available (CLI-008)
  - Checks git tag vs package.json version for consistency
  - Queries npm registry for @sengac/fspec current version
  - Conditionally runs npm publish only if version differs

  Critical implementation requirements:
  - MUST verify git tag matches package.json version first
  - MUST check npm registry before publishing
  - MUST exit successfully (not error) if version already published
  - MUST NOT run tests or build (assumes /release already validated)

  Dependencies:
  - Git repository with tags
  - package.json with version field
  - npm registry access for @sengac/fspec
  - /release command (CLI-008)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Implement as slash command for Claude Code in .claude/commands/publish.md (NOT an fspec CLI command)
  #   2. Verify git tag matches package.json version before comparing with npm registry (fail if mismatch)
  #   3. Compare package.json version with npm registry for @sengac/fspec - only publish if different
  #   4. If version already exists on npm registry, show message and exit successfully (not an error)
  #   5. No pre-publish testing or building - assume /release already validated. Only check version differences before publishing.
  #
  # EXAMPLES:
  #   1. Git tag: v0.3.1, package.json: 0.3.1, npm registry: 0.3.0. Versions match locally, differs from npm. Command proceeds with npm publish.
  #   2. Git tag: v0.3.1, package.json: 0.3.0, npm registry: 0.3.0. Mismatch detected. Command fails with error: 'Version mismatch: git tag (v0.3.1) does not match package.json (0.3.0). Run /release first.'
  #   3. Git tag: v0.3.0, package.json: 0.3.0, npm registry: 0.3.0. Command shows: 'Version 0.3.0 already published to npm. Skipping.' and exits successfully (exit code 0).
  #   4. User runs /publish after /release. Command checks: git tag (v0.3.1) matches package.json (0.3.1), differs from npm (0.3.0). Runs npm publish directly without tests/build.
  #
  # ========================================
  Background: User Story
    As a developer using Claude Code
    I want to publish releases to npm only if version differs from registry
    So that I prevent accidental duplicate publishing

  Scenario: Publish new version when git tag and package.json match and differ from npm
    Given the current git tag is "v0.3.1"
    And package.json version is "0.3.1"
    And npm registry shows version "0.3.0" for @sengac/fspec
    When Claude runs "/publish" command
    Then git tag and package.json versions should be verified as matching
    And the npm registry version should be checked
    And "npm publish" should be executed
    And the package should be published to npm

  Scenario: Fail when git tag and package.json versions mismatch
    Given the current git tag is "v0.3.1"
    And package.json version is "0.3.0"
    And npm registry shows version "0.3.0" for @sengac/fspec
    When Claude runs "/publish" command
    Then the command should fail with error
    And the error should be "Version mismatch: git tag (v0.3.1) does not match package.json (0.3.0). Run /release first."
    And "npm publish" should NOT be executed

  Scenario: Skip publishing when version already exists on npm
    Given the current git tag is "v0.3.0"
    And package.json version is "0.3.0"
    And npm registry shows version "0.3.0" for @sengac/fspec
    When Claude runs "/publish" command
    Then the command should show message "Version 0.3.0 already published to npm. Skipping."
    And the command should exit successfully (exit code 0)
    And "npm publish" should NOT be executed

  Scenario: Publish after successful release without tests or build
    Given Claude has run "/release" successfully
    And the current git tag is "v0.3.1"
    And package.json version is "0.3.1"
    And npm registry shows version "0.3.0" for @sengac/fspec
    When Claude runs "/publish" command
    Then "npm test" should NOT be executed
    And "npm run build" should NOT be executed
    And "npm publish" should be executed directly
