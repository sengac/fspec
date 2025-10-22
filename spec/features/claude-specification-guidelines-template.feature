@done
@INIT-003
@critical
@cli
@initialization
@documentation
@template
Feature: CLAUDE.md Specification Guidelines Template
  """
  Architecture notes:
  - Entire spec/ and .claude/ directories bundled with fspec package
  - CLAUDE.md copied during 'fspec init' command execution
  - Located at spec/CLAUDE.md (entire spec/ dir bundled to dist/spec/)
  - Copied to target project's spec/CLAUDE.md
  - Always overwrites existing file (no version checking, no prompts)
  - No separate confirmation message (implicit in workflow)

  Critical implementation requirements:
  - MUST bundle entire spec/ directory (all .md, .json, .feature files)
  - MUST bundle entire .claude/ directory (fspec.md and other files)
  - MUST resolve path from package installation directory
  - MUST copy CLAUDE.md exactly (no modifications or customization)
  - MUST create spec/ directory if it doesn't exist
  - MUST overwrite existing spec/CLAUDE.md without prompting

  Implementation approach:
  - Enhance existing src/commands/init.ts
  - Add copyClaudeTemplate() function
  - Read spec/CLAUDE.md from bundled package installation
  - Use fs.copyFile() for exact copy

  Build configuration:
  - Vite plugin uses cpSync to recursively copy directories
  - Copies spec/ → dist/spec/ (all subdirectories and files)
  - Copies .claude/ → dist/.claude/ (all subdirectories and files)
  - Template resolution tries: dist/spec/CLAUDE.md, spec/CLAUDE.md
  - All feature files, JSON configs, and markdown docs included

  References:
  - Command: src/commands/init.ts
  - Source: spec/CLAUDE.md (part of bundled spec/ directory)
  - Slash command: .claude/commands/fspec.md (part of bundled .claude/ directory)
  - Build: vite.config.ts (copy-bundled-files plugin)
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
  Scenario: spec/CLAUDE.md is bundled with package
    Given I have installed fspec as npm package
    When I inspect the package installation directory
    Then dist/spec/ directory should exist
    And dist/spec/CLAUDE.md should exist
    And the file should be readable by init command
