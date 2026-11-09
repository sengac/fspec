@done
@breaking-change
@work-management
@cli
@high
@CLI-007
Feature: Replace generic create-work-unit with type-specific commands
  """
  Commander.js registration updated in main CLI file - 3 new commands registered, old command removed
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The generic create-work-unit command must be completely removed (breaking change)
  #   2. Each command must emit a type-specific system-reminder after creation guiding AI to the appropriate workflow
  #   3. create-bug must guide AI to research commands: context (CLI-019), search-implementation, show-coverage
  #   4. create-story must guide AI to Example Mapping commands: add-rule, add-example, add-question, set-user-story
  #   5. create-task must indicate minimal requirements (optional feature file, optional tests)
  #   6. The generic create-work-unit command must be completely removed (breaking change)
  #   7. Each command must emit type-specific system-reminder after creation guiding AI to appropriate workflow
  #   8. create-story must guide AI to Example Mapping commands (add-rule, add-example, add-question, set-user-story)
  #   9. create-bug must guide AI to research commands (context, search-implementation, show-coverage)
  #   10. create-task must indicate minimal requirements (optional feature file, optional tests)
  #   11. The group help in src/help.ts must be updated to reference create-story/bug/task instead of create-work-unit
  #   12. All command help files in src/commands/*-help.ts must be updated to remove create-work-unit references and add new command help files
  #   13. README.md must be updated to replace all create-work-unit examples with type-specific command examples
  #   14. spec/AGENTS.md (auto-generated) must be regenerated to reflect new commands in all workflow examples
  #   15. All documentation files in docs/ directory must be searched and updated to replace create-work-unit with type-specific commands
  #   16. All system-reminders in source code must be found and updated to reference new commands (search for 'create-work-unit' in all .ts files)
  #   17. All test files must be updated to test new commands and verify old command is removed
  #   18. Commander.js registration in main CLI file must remove create-work-unit and register 3 new commands
  #   19. Complete example commands (Option A) - more helpful for AI agents because they provide concrete patterns, reduce cognitive load, and enable immediate action
  #   20. All options supported (Option A) - create-story/bug/task all support --epic, --description, --parent for consistency and because all types can legitimately have hierarchical relationships
  #   21. ALL mentions of 'work unit' terminology must be replaced with type-specific terminology (stories, bugs, tasks) throughout the ENTIRE codebase
  #   22. File names containing 'work-unit' must be evaluated for renaming to 'work-item' or type-specific names
  #   23. Hard breaking change (Option A) - v2.0.0 immediately removes create-work-unit and all work unit terminology. Clean break, no deprecation period.
  #   24. Variables named 'workUnit' must be renamed to contextually appropriate names (story, bug, task, or workUnit for generic cases)
  #   25. User-facing text saying 'work unit' must be replaced with 'story', 'bug', 'task', or collective term like 'work units'
  #
  # EXAMPLES:
  #   1. AI runs 'fspec create-story AUTH "User login"' and sees system-reminder: 'Next steps - Example Mapping: 1. fspec add-rule AUTH-001 ..., 2. fspec add-example AUTH-001 ...'
  #   2. AI runs 'fspec create-bug BUG "Login validation broken"' and sees system-reminder: 'CRITICAL: Research existing code FIRST: 1. fspec context --text "login", 2. fspec search-implementation --function="validateLogin"'
  #   3. AI runs 'fspec create-task TASK "Setup CI/CD pipeline"' and sees system-reminder: 'Task created. Tasks can skip feature files and tests for operational work.'
  #   4. AI runs 'fspec create-story AUTH "User login"' and sees system-reminder: 'Next steps - Example Mapping: 1. fspec add-rule AUTH-001 ..., 2. fspec add-example AUTH-001 ...'
  #   5. AI runs 'fspec create-bug BUG "Login validation broken"' and sees system-reminder: 'CRITICAL: Research existing code FIRST: 1. fspec context --text "login", 2. fspec search-implementation --function="validateLogin"'
  #   6. AI runs 'fspec create-task TASK "Setup CI/CD pipeline"' and sees system-reminder: 'Task created. Tasks can skip feature files and tests for operational work.'
  #   7. In src/help.ts, replace 'fspec create-work-unit PREFIX "Title"' with 'fspec create-story PREFIX "Title"' in work management help
  #   8. Delete src/commands/create-work-unit-help.ts completely (breaking change)
  #   9. In README.md Quick Start section, replace 'fspec create-work-unit AUTH "Login"' with 'fspec create-story AUTH "Login"'
  #   10. Search all .ts files for 'create-work-unit' string and update to appropriate type-specific command based on context
  #   11. In spec/AGENTS.md examples, replace all 'fspec create-work-unit' occurrences with context-appropriate create-story/bug/task
  #   12. Grep for system-reminder tags containing 'create-work-unit' and update each to reference new commands with type guidance
  #   13. Replace 'Work unit CLI-007' with 'Story CLI-007' in show-work-unit output
  #   14. Rename function updateWorkUnitStatus() to updateWorkUnitStatus() for generic cases, or updateStoryStatus() for type-specific
  #   15. In docs, change 'work unit management' to 'work unit management' or 'managing stories, bugs, and tasks'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the system-reminders include complete example commands or just command templates?
  #   A: true
  #
  #   Q: Should create-story/bug/refactor/task support all the same options as create-work-unit (--epic, --description, --parent)?
  #   A: true
  #
  #   Q: When AI tries to use the old create-work-unit command, should it show a migration guide in the error message?
  #   A: true
  #
  #   Q: Should there be a deprecation period with warnings, or is this a hard breaking change (v2.0) that immediately removes create-work-unit?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No migration guide (Option B) - Command should simply not exist. AI will adapt by using available commands.
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to create work units with appropriate guidance for each type
    So that I follow the correct discovery workflow for stories, bugs, and tasks

  Scenario: Old create-work-unit command is removed
    Given I have fspec v2.0 installed
    When I run fspec with the removed command 'create-work-unit AUTH "Login"'
    Then the command should fail with the error "error: unknown command 'create-work-unit'"
    And the exit code should be 1
    And no spec/work-units.json should be written
