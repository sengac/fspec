@feature-management
@high
@tui
@cli
@interactive-cli
@ink
@typescript
@ITF-003
Feature: Launch interactive TUI when running fspec with no arguments
  """
  Architecture notes:
  - Entry point: Modify src/index.ts to detect no arguments (process.argv.length === 2)
  - TTY detection: Check process.stdin.isTTY and process.env.CI before launching TUI
  - Error handling: Show error message in CI/non-TTY environments (exit code 1)
  - DRY principle: Reuse existing BoardView component from src/tui/components/BoardView.tsx
  - Rendering: Use Ink's render() function to launch TUI (only in TTY environments)
  - Exit handling: Add onExit prop to BoardView that calls process.exit(0)
  - Dependencies: Ink (already installed), React (already installed)
  - No new external dependencies required (reuse existing TUI infrastructure)

  Critical implementation requirements:
  - MUST check process.stdin.isTTY before launching TUI (Ink requires raw mode)
  - MUST check process.env.CI === 'true' to detect CI environments
  - MUST show helpful error message in non-TTY: "Interactive TUI requires a TTY environment"
  - MUST exit with code 1 when TUI cannot launch (non-TTY/CI)
  - MUST NOT interfere with --help or other commands
  - MUST preserve all existing command functionality

  Testing constraints:
  - Automated tests run in CI/non-TTY environments (cannot test actual TUI rendering)
  - Tests verify TTY detection and error handling (not actual BoardView rendering)
  - Manual testing in real terminal required to verify TUI launches correctly
  - Tests use child_process.spawn() to test CLI behavior
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Running 'fspec' with no arguments MUST launch the interactive TUI in TTY environments
  #   2. Running 'fspec' in CI/non-TTY MUST show error message and exit with code 1
  #   3. Running 'fspec --help' MUST still show help text (not launch TUI)
  #   4. Running 'fspec help' MUST still show help text (not launch TUI)
  #   5. Running 'fspec <command>' MUST execute the command (not launch TUI)
  #   6. TUI MUST be rendered using existing BoardView component (DRY - reuse BOARD-002 implementation)
  #   7. TUI MUST exit cleanly when user presses ESC or q key
  #   8. TTY detection MUST use process.stdin.isTTY and process.env.CI checks
  #
  # EXAMPLES:
  #   1. Developer runs 'fspec' in terminal → TTY detected → BoardView TUI launches → Shows Kanban board
  #   2. CI runs 'fspec' in pipeline → Non-TTY detected → Shows error "Interactive TUI requires a TTY environment" → Exit code 1
  #   3. Developer runs 'fspec --help' → Help text displays → TUI does NOT launch
  #   4. Developer runs 'fspec validate' → Validation executes → TUI does NOT launch
  #   5. Developer in TUI presses ESC → TUI exits cleanly → Returns to shell prompt → Exit code 0
  #   6. Automated tests run with CI=true → Tests verify error handling → Cannot test actual TUI rendering
  #
  # ========================================
  Background: User Story
    As a developer using fspec
    I want to see my work units visually when I run fspec
    So that I get immediate visual feedback and can navigate my work interactively

  Scenario: Launch TUI when running fspec with no arguments in TTY environment
    Given I am in a project directory with fspec initialized
    When I run "fspec" with no arguments in a TTY environment
    Then the interactive TUI should launch
    And the BoardView component should be rendered
    And I should see the Kanban board with work units

  Scenario: Show error when running fspec with no arguments in CI/non-TTY environment
    Given I am in a project directory with fspec initialized
    When I run "fspec" with no arguments in a CI environment
    Then the command should exit with code 1
    And stderr should contain "Interactive TUI requires a TTY environment"
    And stderr should contain "Run with a command or use --help"
    And the TUI should NOT launch
    And stdout should NOT contain "USAGE"

  Scenario: Show help when running fspec --help
    Given I am in a project directory with fspec initialized
    When I run "fspec --help"
    Then help text should be displayed
    And stdout should contain "USAGE"
    And stdout should contain "fspec [command] [options]"
    And the TUI should NOT launch
    And stdout should NOT contain "Interactive TUI requires a TTY environment"

  Scenario: Execute commands normally when arguments provided
    Given I am in a project directory with fspec initialized
    When I run "fspec validate"
    Then the validate command should execute
    And stdout or stderr should match "valid|feature file"
    And the TUI should NOT launch
    And stdout or stderr should NOT contain "Interactive TUI requires a TTY environment"

  Scenario: Exit with error in CI when trying to run TUI
    Given I try to run the TUI in a CI environment
    When I run "fspec" with no arguments and CI=true
    Then the process should exit with code 1
    And stderr should contain "Interactive TUI requires a TTY environment"
    And the process should exit immediately without launching TUI
