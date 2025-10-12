@done
@INIT-003
@phase1
@critical
@cli
@initialization
@documentation
@template
Feature: CLAUDE.md Specification Guidelines Template
  """
  Architecture notes:
  - CLAUDE.md template bundled with fspec package in templates/ directory
  - Template copied during 'fspec init' command execution
  - Located at templates/CLAUDE.md (bundled in package build)
  - Copied to spec/CLAUDE.md in target project
  - Always overwrites existing file (no version checking, no prompts)
  - Confirmation message shown in /fspec slash command output only

  Critical implementation requirements:
  - MUST bundle templates/CLAUDE.md in package build (include in dist/)
  - MUST resolve template path from package installation directory
  - MUST copy file exactly (no modifications or customization)
  - MUST create spec/ directory if it doesn't exist
  - MUST overwrite existing spec/CLAUDE.md without prompting
  - MUST NOT show separate CLI message (only in /fspec output)

  Implementation approach:
  - Enhance existing src/commands/init.ts
  - Add copyClaudeTemplate() function
  - Import templates/CLAUDE.md from package installation
  - Use fs.copyFile() for exact copy
  - Update /fspec command template to show confirmation

  Build configuration:
  - Ensure templates/ directory included in build output
  - Vite config must copy templates/ to dist/templates/
  - Template resolution: __dirname/../templates/CLAUDE.md

  References:
  - Command: src/commands/init.ts
  - Template: templates/CLAUDE.md (to be created)
  - Slash command: .claude/commands/fspec.md
  """

  Background: User Story
    As a developer setting up a new project with fspec
    I want the CLAUDE.md specification guidelines automatically copied to my project
    So that I have complete documentation of the ACDD workflow and fspec best practices

  @critical
  @happy-path
  Scenario: Init command copies CLAUDE.md template to new project
    Given I have an empty directory for a new project
    And spec/CLAUDE.md does not exist
    When I run "fspec init"
    Then .claude/commands/fspec.md should be created
    And spec/CLAUDE.md should be created
    And spec/CLAUDE.md should contain specification guidelines
    And the file should be an exact copy of templates/CLAUDE.md

  @critical
  @overwrite
  Scenario: Init command overwrites existing CLAUDE.md without prompting
    Given I have a project with existing spec/CLAUDE.md
    And the existing file contains outdated content
    When I run "fspec init"
    Then spec/CLAUDE.md should be overwritten with latest template
    And no backup file should be created
    And no overwrite prompt should be shown

  @confirmation
  Scenario: Slash command output confirms CLAUDE.md was copied
    Given I run "fspec init" in a new project
    When I view the /fspec slash command in Claude Code
    Then the output should mention "Copied spec/CLAUDE.md specification guidelines"
    And the output should explain the purpose of CLAUDE.md

  @edge-case
  Scenario: Init creates spec directory if missing
    Given I have a project without spec/ directory
    When I run "fspec init"
    Then spec/ directory should be created
    And spec/CLAUDE.md should be copied to the new directory
    And the command should succeed without errors

  @validation
  Scenario: Template file is identical across all projects
    Given I run "fspec init" in project A
    And I run "fspec init" in project B
    When I compare spec/CLAUDE.md from both projects
    Then both files should be byte-for-byte identical
    And no project-specific customization should exist

  @build
  Scenario: Templates directory is bundled with package
    Given I have installed fspec as npm package
    When I inspect the package installation directory
    Then templates/ directory should exist
    And templates/CLAUDE.md should exist
    And the template should be readable by init command
