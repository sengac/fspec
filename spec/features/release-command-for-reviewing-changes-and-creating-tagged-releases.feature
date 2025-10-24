@done
@automation
@release-management
@cli
@high
@CLI-008
Feature: Release command for reviewing changes and creating tagged releases
  """
  Architecture notes:
  - Implemented as Claude Code slash command in .claude/commands/release.md
  - Uses git log to review commits since last tag
  - Uses git diff to analyze line-by-line code changes
  - Determines semver bump from conventional commits (BREAKING CHANGE = major, feat = minor, fix = patch)
  - Updates package.json version field before creating release commit
  - Creates git tag matching the new version
  - Does NOT push to remote (manual step)

  Critical implementation requirements:
  - Commit author MUST be Roland Quast <rquast@rolandquast.com> (not Claude's default)
  - Release commit format: 'chore(release): v{version}' with structured release notes
  - Pre-release validation: stage uncommitted files, run npm test, run npm run build
  - If tests or build fail, abort release with error
  - Generate conventional commit message when committing uncommitted changes (not generic)

  Dependencies:
  - Git repository with commit history
  - package.json with version field
  - npm test script for validation
  - npm run build script for validation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Implement as slash command for Claude Code in .claude/commands/release.md (NOT an fspec CLI command)
  #   2. Commit author must be Roland Quast <rquast@rolandquast.com>, NOT Claude's default author
  #   3. Review all git commits since last tag using git log
  #   4. Analyze code changes in those commits for release notes
  #   5. Create git tag for the release version
  #   6. Do NOT push to remote (user must push manually)
  #   7. Use conventional commits to determine semver bump: BREAKING CHANGE = major, feat = minor, fix = patch
  #   8. Review ALL code changes line-by-line since last tag (not just commit messages)
  #   9. Combine conventional commit analysis with actual code diff review to generate comprehensive release notes
  #   10. Update package.json version field to match the new release version
  #   11. Release commit message format: 'chore(release): v{version}' with full release notes in commit body (breaking changes, features, fixes sections)
  #   12. If no previous git tag exists, use current package.json version as base for semver bump
  #   13. Before creating release: 1) Stage and commit all uncommitted changes with descriptive message, 2) Run npm test, 3) Run npm run build, 4) Abort release if tests or build fail
  #   14. When committing uncommitted changes before release: analyze changes and generate appropriate conventional commit message (not generic message)
  #
  # EXAMPLES:
  #   1. Last tag is v0.3.0. Commits since: 'feat: add new command', 'fix: resolve bug'. Code diff shows 500 lines added. Claude reviews line-by-line, determines minor bump (feat), creates v0.3.1 tag with release notes combining commit analysis + code review.
  #   2. Last tag v0.3.0, new version v0.3.1 determined. Command updates package.json from 'version: 0.3.0' to 'version: 0.3.1', creates commit with updated package.json, then creates v0.3.1 tag.
  #   3. Commit message: 'chore(release): v0.3.1' with body containing '## Breaking Changes\n(none)\n\n## Features\n- Add new command\n\n## Fixes\n- Resolve critical bug'
  #   4. No previous tag. package.json shows 'version: 0.3.0'. Commits contain 'feat: new feature'. Bump to v0.4.0 (minor bump from package.json version).
  #   5. User runs /release with uncommitted files. Command commits changes first, runs npm test (passes), runs npm run build (passes), then proceeds with release. If either fails, release is aborted with error message.
  #   6. Uncommitted changes include new test file and bug fix. Pre-release commit message: 'fix: resolve authentication bug and add test coverage' before proceeding with release commit.
  #
  # ========================================
  Background: User Story
    As a developer using Claude Code
    I want to create tagged releases with comprehensive release notes
    So that I can automate version bumping and release documentation based on conventional commits

  Scenario: Determine semver bump from conventional commits and create release
    Given the last git tag is "v0.3.0"
    And there are commits since the tag: "feat: add new command", "fix: resolve bug"
    And the code diff shows 500 lines added
    When Claude runs "/release" command
    Then Claude should review all commits since last tag
    And Claude should analyze the code diff line-by-line
    And Claude should determine the version bump as minor (due to "feat:" commit)
    And Claude should create git tag "v0.3.1"
    And the release notes should combine commit analysis and code review

  Scenario: Update package.json version and create release commit
    Given the last git tag is "v0.3.0"
    And package.json shows version "0.3.0"
    And the new version determined is "v0.3.1"
    When the release command updates package.json
    Then package.json version field should change from "0.3.0" to "0.3.1"
    And a commit should be created with the updated package.json
    And the git tag "v0.3.1" should be created

  Scenario: Format release commit message with structured release notes
    Given the new release version is "v0.3.1"
    And there are features: "Add new command"
    And there are fixes: "Resolve critical bug"
    And there are no breaking changes
    When the release commit is created
    Then the commit message should start with "chore(release): v0.3.1"
    And the commit body should contain "## Breaking Changes\n(none)"
    And the commit body should contain "## Features\n- Add new command"
    And the commit body should contain "## Fixes\n- Resolve critical bug"
    And the commit author should be "Roland Quast <rquast@rolandquast.com>"

  Scenario: Handle first release when no previous tag exists
    Given no previous git tag exists
    And package.json shows version "0.3.0"
    And there are commits with "feat: new feature"
    When Claude runs "/release" command
    Then the base version should be "0.3.0" from package.json
    And the version bump should be minor (due to "feat:" commit)
    And the new version should be "v0.4.0"
    And git tag "v0.4.0" should be created

  Scenario: Pre-release validation with uncommitted changes
    Given there are uncommitted files in the working directory
    When Claude runs "/release" command
    Then all uncommitted changes should be staged and committed with a conventional commit message
    And "npm test" should be executed
    And "npm run build" should be executed
    And if tests pass and build succeeds, the release should proceed
    And if tests fail or build fails, the release should abort with an error message

  Scenario: Generate conventional commit message for uncommitted changes
    Given there are uncommitted changes: new test file and bug fix code
    When the pre-release commit is created
    Then the commit message should be "fix: resolve authentication bug and add test coverage"
    And the commit message should NOT be a generic message
    And the commit should be created before the release commit
