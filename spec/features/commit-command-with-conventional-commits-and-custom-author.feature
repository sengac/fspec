@done
@high
@cli
@release-management
@automation
@CLI-010
Feature: Commit command with conventional commits and custom author
  """
  Architecture notes:
  - Implemented as Claude Code slash command in .claude/commands/commit.md
  - Analyzes unstaged and staged files to determine change type
  - Generates conventional commit message (type(scope): description)
  - Stages all unstaged files before committing (git add .)
  - Does NOT push to remote (manual step)

  Critical implementation requirements:
  - Commit author MUST be Roland Quast <rquast@rolandquast.com> (not Claude's default)
  - Follow strict conventional commits spec: type(scope): description
  - Include body with detailed changes
  - Include footer with BREAKING CHANGE if applicable
  - If clean working directory, show message and exit successfully (not error)

  Dependencies:
  - Git repository with working directory
  - Claude's code analysis capabilities for commit message generation
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Implement as slash command for Claude Code in .claude/commands/commit.md (NOT an fspec CLI command)
  #   2. Commit author must be Roland Quast <rquast@rolandquast.com>, NOT Claude's default author
  #   3. Stage all unstaged files before committing (git add .)
  #   4. Generate conventional commit message by analyzing staged files and changes
  #   5. Do NOT push to remote (user must push manually)
  #   6. If no unstaged or staged files exist (clean working directory), show message and exit successfully (not an error)
  #   7. Follow strict conventional commits spec: type(scope): description. Include body with detailed changes and footer with BREAKING CHANGE if applicable.
  #
  # EXAMPLES:
  #   1. User runs /commit with clean working directory. Command shows: 'Nothing to commit, working directory clean.' and exits successfully.
  #   2. Unstaged files: src/commands/new-feature.ts (new file), src/types.ts (modified). Generated commit: 'feat(commands): add new feature command\n\nImplement new-feature command with TypeScript types and CLI integration.'
  #
  # ========================================
  Background: User Story
    As a developer using Claude Code
    I want to create conventional commits with automated message generation
    So that I maintain consistent commit history without manual message writing

  Scenario: Handle clean working directory gracefully
    Given the working directory is clean
    And there are no unstaged files
    And there are no staged files
    When Claude runs "/commit" command
    Then the command should show message "Nothing to commit, working directory clean."
    And the command should exit successfully (exit code 0)
    And no commit should be created

  Scenario: Generate conventional commit for new feature with multiple files
    Given there are unstaged files: "src/commands/new-feature.ts" (new file)
    And there are unstaged files: "src/types.ts" (modified)
    When Claude runs "/commit" command
    Then all unstaged files should be staged with "git add ."
    And Claude should analyze the changes to determine commit type
    And the commit message should be "feat(commands): add new feature command"
    And the commit body should contain "Implement new-feature command with TypeScript types and CLI integration."
    And the commit author should be "Roland Quast <rquast@rolandquast.com>"
    And the changes should be committed

  Scenario: Generate conventional commit for bug fix
    Given there are unstaged files: "src/commands/validate.ts" (modified - bug fix)
    When Claude runs "/commit" command
    Then the commit type should be "fix"
    And the commit message should follow format "fix(validation): [description]"
    And the commit should include detailed changes in the body
    And the commit author should be "Roland Quast <rquast@rolandquast.com>"

  Scenario: Generate conventional commit with breaking change
    Given there are unstaged files with breaking API changes
    When Claude runs "/commit" command
    Then the commit message should include "BREAKING CHANGE:" in the footer
    And the commit body should describe the breaking changes
    And the commit should follow conventional commits spec
    And the commit author should be "Roland Quast <rquast@rolandquast.com>"

  Scenario: Do not push to remote after commit
    Given there are unstaged files to commit
    When Claude runs "/commit" command
    And the commit is created successfully
    Then "git push" should NOT be executed
    And the user must push manually
