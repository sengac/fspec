@CLI-004
@DOC-001
@done
@scaffolding
@phase1
@cli
@setup
Feature: fspec Slash Command Installation
  """
  Architecture notes:
  - Uses ink/react for interactive prompts (consistent with board UI)
  - Template based on .claude/commands/fspec.md with generic placeholders
  - Path validation prevents escaping current directory (security)
  - Creates parent directories automatically using fs.mkdir with recursive: true
  - File operations use fs/promises for async I/O
  - Exit codes: 0 for success or user cancellation, non-zero for errors

  Critical implementation requirements:
  - MUST prompt user to choose: Claude Code or custom location
  - MUST validate custom paths are relative (not parent or absolute paths)
  - MUST create parent directories if they don't exist
  - MUST prompt for confirmation before overwriting existing files
  - MUST use ink/react for interactive UI (consistent with board command)
  - MUST generate template with lowercase example placeholders (example-project style)
  - MUST include complete template content from current fspec.md
  - MUST display both success message and next steps on completion

  Template transformation:
  - Replace fspec-specific examples with generic placeholders
  - Keep all sections: ACDD workflow, example mapping, estimation, etc
  - Use example-project, example-feature naming patterns
  - Preserve all instructions and workflow steps intact
  """

  Background: User Story
    As a developer using fspec in an AI-assisted project
    I want to run `fspec init` to install the /fspec slash command
    So that I can activate fspec mode in Claude Code or other AI tools

  Scenario: Install to Claude Code default location
    Given I am in a project directory
    When I run `fspec init`
    And I select "Claude Code" from the installation options
    Then the file should be created at ".claude/commands/fspec.md"
    And the file should contain the complete generic template
    And the output should display "✓ Installed /fspec command to .claude/commands/fspec.md"
    And the output should display "Run /fspec in Claude Code to activate"

  Scenario: Install to custom location
    Given I am in a project directory
    When I run `fspec init`
    And I select "Custom location" from the installation options
    And I enter "docs/ai/fspec.md" as the file path
    Then the file should be created at "docs/ai/fspec.md"
    And the parent directory "docs/ai" should be created if it doesn't exist
    And the file should contain the complete generic template
    And the output should display "✓ Installed /fspec command to docs/ai/fspec.md"

  Scenario: Overwrite existing file with confirmation
    Given I am in a project directory
    And the file ".claude/commands/fspec.md" already exists
    When I run `fspec init`
    And I select "Claude Code" from the installation options
    And I confirm the overwrite prompt
    Then the file should be overwritten at ".claude/commands/fspec.md"
    And the output should display "✓ Installed /fspec command to .claude/commands/fspec.md"

  Scenario: Cancel overwrite of existing file
    Given I am in a project directory
    And the file ".claude/commands/fspec.md" already exists
    When I run `fspec init`
    And I select "Claude Code" from the installation options
    And I decline the overwrite prompt
    Then the file should not be modified
    And the output should display "Installation cancelled"
    And the command should exit with code 0

  Scenario: Reject path escaping current directory
    Given I am in a project directory
    When I run `fspec init`
    And I select "Custom location" from the installation options
    And I enter "../parent/fspec.md" as the file path
    Then the command should display an error "Path must be relative to current directory"
    And the command should exit with non-zero code
    And no file should be created

  Scenario: Reject absolute path
    Given I am in a project directory
    When I run `fspec init`
    And I select "Custom location" from the installation options
    And I enter "/absolute/path/fspec.md" as the file path
    Then the command should display an error "Path must be relative to current directory"
    And the command should exit with non-zero code
    And no file should be created

  Scenario: Handle file write errors gracefully
    Given I am in a project directory
    And I do not have write permissions for ".claude/commands/"
    When I run `fspec init`
    And I select "Claude Code" from the installation options
    Then the command should display an error message about permission denied
    And the command should exit with non-zero code

  Scenario: Template contains generic placeholders
    Given I am in a project directory
    When I run `fspec init`
    And I select "Claude Code" from the installation options
    Then the generated template should contain "example-project" style placeholders
    And the template should not contain fspec-specific examples
    And the template should include all ACDD workflow sections
    And the template should include example mapping guidance
    And the template should include story point estimation guidance

  Scenario: CLI registration allows running fspec init command
    Given I have fspec installed
    When I run 'fspec init --help'
    Then the command should show init command help
    And the command should not show 'unknown command' error
    And 'fspec --help' output should list init command
