@critical
@demos
@video-recording
@bug
Feature: Demo scripts incorrectly try to import non-existent TestController class
  """

  Key architectural decisions:
  - @microsoft/tui-test is a TEST FRAMEWORK (like Jest/Vitest for terminals)
  - Provides test() and expect() functions for defining terminal tests
  - Terminal instance is passed as a fixture to test functions
  - Uses node-pty for PTY management and @xterm/headless for terminal emulation

  Dependencies and integrations:
  - @microsoft/tui-test (test framework)
  - node-pty (PTY spawning)
  - @xterm/headless (terminal emulation)
  - Jest expect library (assertions)

  Critical implementation requirements:
  - MUST convert existing demos from standalone scripts to .test.ts test files
  - MUST replace import { TestController } with import { test, expect }
  - MUST wrap demo logic in test('name', async ({ terminal }) => { ... })
  - MUST use terminal fixture methods (submit, write, key*) instead of spawn/controller
  - MUST use await expect(terminal.getByText(...)).toBeVisible() for synchronization
  - MUST update recorder to execute test files with tui-test CLI
  - See attached docs/tui-test-usage-guide.md for complete migration guide

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Demo scripts MUST be written as .test.ts files using test() and expect() from @microsoft/tui-test
  #   2. Demo scripts MUST access terminal via fixture parameter: test('name', async ({ terminal }) => ...)
  #   3. Demo scripts MUST use terminal.submit(), terminal.write(), terminal.key*() methods (not spawn/controller)
  #   4. Demo scripts MUST use await expect(terminal.getByText(...)).toBeVisible() for waiting and assertions
  #   5. Demo scripts MUST be run with tui-test CLI, not npx tsx or node directly
  #
  # EXAMPLES:
  #   1. Shell demo: test.use({ shell: Shell.Bash }); test('demo', async ({ terminal }) => { terminal.submit('ls'); await expect(terminal.getByText('package.json')).toBeVisible(); });
  #   2. Program demo: test.use({ program: { file: 'node', args: ['app.js'] } }); test('demo', async ({ terminal }) => { await expect(terminal.getByText('Started')).toBeVisible(); });
  #   3. Navigation demo: terminal.keyDown(); terminal.keyDown(); terminal.submit(); // press Enter
  #   4. fspec demo: test.use({ program: { file: 'node', args: ['~/projects/fspec/dist/index.js'] } }); test('demo', async ({ terminal }) => { await expect(terminal.getByText('fspec')).toBeVisible(); terminal.keyDown(); });
  #
  # ========================================
  Background: User Story
    As a developer using fspec.videos
    I want to write demo scripts using @microsoft/tui-test framework
    So that I can create working demo videos of terminal applications

  Scenario: Demo script uses shell with terminal fixture
    Given I have @microsoft/tui-test installed
    When I create a demo script as a .test.ts file
    And I import test and expect from "@microsoft/tui-test"
    And I use test.use({ shell: Shell.Bash }) to configure shell
    And I write test('demo', async ({ terminal }) => { ... }) with terminal fixture
    And I use terminal.submit('ls') to execute command
    And I use await expect(terminal.getByText('package.json')).toBeVisible() for assertion
    Then the demo script should execute successfully with tui-test CLI
    And the terminal should show the command output

  Scenario: Demo script runs a program with terminal fixture
    Given I have @microsoft/tui-test installed
    When I create a demo script as a .test.ts file
    And I use test.use({ program: { file: 'node', args: ['app.js'] } }) to run program
    And I write test('demo', async ({ terminal }) => { ... }) with terminal fixture
    And I use await expect(terminal.getByText('Started')).toBeVisible() to wait for output
    Then the demo script should spawn the program
    And the terminal should show program output
    And assertions should pass

  Scenario: Demo script uses keyboard navigation methods
    Given I have a demo script with terminal fixture
    When I use terminal.keyDown() to press down arrow
    And I use terminal.keyDown() again to press down arrow
    And I use terminal.submit() to press Enter key
    Then the terminal should receive the keyboard inputs
    And the application should respond to navigation
    And the demo should record the keyboard interactions

  Scenario: Demo script controls fspec TUI application
    Given I have @microsoft/tui-test installed
    When I create a demo script as a .test.ts file
    And I use test.use({ program: { file: 'node', args: ['~/projects/fspec/dist/index.js'] } })
    And I write test('demo', async ({ terminal }) => { ... }) with terminal fixture
    And I use await expect(terminal.getByText('fspec')).toBeVisible() to wait for TUI
    And I use terminal.keyDown() to navigate in fspec
    Then the demo script should control the fspec TUI
    And the terminal should show fspec interface
    And navigation should work correctly

  Scenario: Existing broken demo script fails with import error
    Given I have an existing demo script using TestController
    And the script contains "import { TestController } from '@microsoft/tui-test'"
    When I try to run the script with npx tsx
    Then the script should fail with import error
    And the error should say "does not provide an export named 'TestController'"
    And the script should not execute

  Scenario: Migrated demo script uses correct tui-test API
    Given I have a broken demo script using TestController
    When I rename the file to .test.ts
    And I replace "import { TestController }" with "import { test, expect }"
    And I wrap the logic in test('name', async ({ terminal }) => { ... })
    And I replace controller.spawn() with test.use({ program: ... })
    And I replace controller.write() with terminal.submit() or terminal.write()
    And I replace controller.waitForText() with await expect(terminal.getByText(...)).toBeVisible()
    Then the demo script should execute successfully
    And the terminal should work correctly
    And the demo should record properly
