@done
@foundation
@discovery
@cli
@FOUND-007
Feature: Foundation existence check in commands
  """
  Architecture notes:
  - Create utility function checkFoundationExists() in src/utils/foundation-check.ts
  - Function checks for spec/foundation.json file existence
  - Function returns error message with system reminder if missing
  - Integrate check into PM commands: board, create-story, create-bug, create-task, update-work-unit-status, create-epic
  - System reminder MUST include original command arguments for retry after discover-foundation
  - Read-only commands (show-foundation, list-features, validate) exempt from check
  - Error message format: User-visible + <system-reminder> block for AI agents
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Commands that manage work units MUST check for foundation.json existence before executing
  #   2. If foundation.json is missing, command MUST exit with error code 1 and display helpful message
  #   3. Error message MUST direct user to run 'fspec discover-foundation' command
  #   4. System reminder MUST tell AI agent to return to original task after discover-foundation completes
  #   5. Check applies to: fspec board, create-story, create-bug, create-task, update-work-unit-status, create-epic, and other PM commands
  #   6. Yes, include exact command arguments. System reminders in Claude Code are used for actionable guidance - including the original command makes it clear what to retry after discover-foundation completes.
  #   7. No, only create/modify commands need foundation.json. Read-only commands like show-foundation, list-features, validate can run without foundation.json.
  #   8. Check only spec/foundation.json - this is the canonical location used by show-foundation, update-foundation, and generate-foundation-md commands.
  #
  # EXAMPLES:
  #   1. AI runs 'fspec board' without foundation.json → Command exits with error, displays message with discover-foundation instructions, system reminder tells AI to run board again after discovery
  #   2. AI runs 'fspec create-story AUTH "Login"' without foundation.json → Command exits with error, system reminder includes original command to retry
  #   3. AI runs 'fspec board' with foundation.json present → Command executes normally, no check triggered
  #   4. AI runs 'fspec validate' (non-PM command) without foundation.json → Command executes normally, foundation check not required for spec-only commands
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the system reminder include the exact command arguments that were originally attempted?
  #   A: true
  #
  #   Q: Should read-only commands like 'fspec show-foundation' also require foundation.json, or only commands that create/modify work units?
  #   A: true
  #
  #   Q: Should the check look for BOTH foundation.json AND spec/foundation.json paths, or just spec/foundation.json?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to ensure foundation.json exists before running project management commands
    So that I prevent work from proceeding without proper project foundation and can resume my original task after discovery

  Scenario: Run board command without foundation.json
    Given I am in a project directory
    And the file "spec/foundation.json" does not exist
    When I run 'fspec board'
    Then the command should exit with code 1
    And the output should display an error message
    And the error message should instruct me to run 'fspec discover-foundation'
    And a system reminder should tell me to retry 'fspec board' after discover-foundation completes

  Scenario: Run create-story without foundation.json
    Given I am in a project directory
    And the file "spec/foundation.json" does not exist
    When I run 'fspec create-story AUTH "Login"'
    Then the command should exit with code 1
    And the output should display an error message
    And the error message should instruct me to run 'fspec discover-foundation'
    And a system reminder should include the original command 'fspec create-story AUTH "Login"' to retry

  Scenario: Run board command with foundation.json present
    Given I am in a project directory
    And the file "spec/foundation.json" exists
    When I run 'fspec board'
    Then the command should execute normally
    And no foundation check error should be displayed

  Scenario: Run validate command without foundation.json (read-only exempt)
    Given I am in a project directory
    And the file "spec/foundation.json" does not exist
    When I run 'fspec validate'
    Then the command should execute normally
    And no foundation check error should be displayed
    And validation should proceed for feature files
