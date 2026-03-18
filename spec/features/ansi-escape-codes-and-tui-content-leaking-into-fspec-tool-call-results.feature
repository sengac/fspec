@BUG-103
Feature: ANSI escape codes and TUI content leaking into Fspec tool call results
  """
  Modifies output.ts createCaptureContext() and fspec-callback.ts capture layers. Adds shared stripAnsi() to output.ts that handles all CSI/OSC/SGR sequences. Removes Layer 3 process.stdout.write override that captured TUI Ink renders concurrently. Propagates Commander configureOutput to all subcommands to capture help/errors without global stdout interception.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All output captured via createCaptureContext() in output.ts must be stripped of ANSI escape codes before storage
  #   2. fspec-callback must NOT override process.stdout.write/stderr.write (TUI Ink renders concurrently and would contaminate tool results)
  #   3. ANSI stripping must handle all CSI sequences (colors, cursor movement, line erasure, mouse tracking), not just SGR color codes
  #   4. The stripAnsi function must be shared (not duplicated) across output.ts and fspec-callback.ts
  #   5. Commander configureOutput must be propagated to all subcommands so help/error output is captured without a global stdout override
  #
  # EXAMPLES:
  #   1. Command passes chalk.green(tag) through output.log() → captured with raw \x1b[32m escape codes in result
  #   2. TUI cursor sequences like \x1b[49A, \x1b[2K, \x1b[E leak through Layer 3 into tool results
  #   3. After fix: output.log(chalk.green('tag')) → captured as plain 'tag' in result
  #   4. After fix: TUI Ink renders go to terminal normally, not captured into tool results
  #
  # ========================================
  Background: User Story
    As a LLM agent
    I want to receive clean text/JSON from fspec tool calls
    So that I can parse results without garbled ANSI escape codes or TUI artifacts

  Scenario: Capture context strips ANSI color codes from output.log
    Given a capture context is created via createCaptureContext
    When a command calls output.log with chalk-formatted text containing SGR escape codes
    Then the captured stdout array should contain only plain text with no ANSI escape sequences

  Scenario: Capture context strips ANSI codes from output.error and output.warn
    Given a capture context is created via createCaptureContext
    When a command calls output.error and output.warn with chalk-formatted text
    Then the captured stderr array should contain only plain text with no ANSI escape sequences

  Scenario: Strip ANSI handles all CSI sequences not just SGR colors
    Given text containing CSI cursor movement, line erasure, and mouse tracking sequences
    When the text is passed through the stripAnsi function
    Then all CSI sequences including cursor up, erase line, next line, and mouse tracking are removed

  Scenario: Commander configureOutput propagated to subcommands
    Given a fresh Commander program with subcommands registered via createProgram
    When configureOutput is called on the program with capture callbacks
    Then all subcommands should also have their output configuration updated
    And subcommand help output should be captured by configureOutput not process.stdout.write

  Scenario: End-to-end fspec callback returns clean output
    Given a project with valid fspec structure
    When fspecCallback is called to execute a command
    Then the returned JSON string should not contain any ANSI escape sequences
