@done
@TUI-100
Feature: Sanitize bash tool output before TUI rendering to prevent terminal trashing
  """
  Create a new module `sanitize.rs` in `rust/fspec-tui/src/store/agent_view/` with a `sanitize_for_terminal(text: &str) -> String` function that mirrors TypeScript's `sanitizeForTerminal()` from `stringWidth.ts`. Uses regex for ANSI stripping and char filtering for control characters.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All tool output text (stdout from bash commands) must be sanitized before being rendered in the TUI scrollback to prevent ANSI escape sequences from trashing the terminal display
  #   2. Sanitization must strip ANSI escape sequences (CSI, OSC, SGR) using a regex equivalent to the TypeScript `sanitizeForTerminal()` function
  #   3. Sanitization must replace tab characters with two spaces for consistent width
  #   4. Sanitization must remove carriage return characters (\r) to prevent line overwriting
  #   5. Sanitization must remove control characters (0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F) except newlines (0x0A) which are preserved
  #   6. Sanitization must be applied at the TUI display layer (fspec-tui chunk processor), NOT at the bash tool layer (rust/tools), so the LLM still receives raw output
  #   7. Sanitization must be applied to both ToolResult content and ToolProgress output chunks before they are stored in the scrollback buffer
  #
  # EXAMPLES:
  #   1. Command `ls --color=always` outputs ANSI color codes like \x1b[01;34m for directories; after sanitization the \x1b[01;34m sequences are removed and only the filenames are displayed
  #   2. Command `grep --color=always 'pattern' file.txt` outputs colored matches; after sanitization the color codes are stripped and only the text content is displayed
  #   3. Command `neofetch` outputs complex ANSI sequences including cursor movement and colors; after sanitization all escape sequences are stripped leaving only readable text
  #   4. Command `echo 'hello\tworld'` outputs a tab character; after sanitization the tab is replaced with two spaces
  #
  # ========================================
  Background: User Story
    As a user running the Rust TUI
    I want to execute bash commands that emit ANSI escape sequences
    So that see clean output without terminal display being trashed

  @unit
  @rust-tui
  @bug-fix
  Scenario: ANSI color codes from ls --color are stripped before rendering
    Given a bash command outputs text with ANSI color escape sequences like "\x1b[01;34m" for colored directories
    When the tool output is processed for TUI display
    Then the ANSI escape sequences are removed from the displayed text
    And only the plain text content (filenames) is visible in the TUI

  @unit
  @rust-tui
  @bug-fix
  Scenario: Complex ANSI sequences from neofetch are fully stripped
    Given a bash command outputs complex ANSI sequences including cursor movement, colors, and bold formatting
    When the tool output is processed for TUI display
    Then all ANSI escape sequences are removed from the displayed text
    And only readable plain text is visible in the TUI

  @unit
  @rust-tui
  @bug-fix
  Scenario: Tab characters in bash output are replaced with spaces
    Given a bash command outputs text containing tab characters
    When the tool output is processed for TUI display
    Then each tab character is replaced with two spaces
    And the text maintains consistent visual width

  @unit
  @rust-tui
  @bug-fix
  Scenario: Carriage returns are removed to prevent line overwriting
    Given a bash command outputs text containing carriage return characters
    When the tool output is processed for TUI display
    Then carriage return characters are removed from the displayed text
    And lines are not overwritten in the TUI

  @unit
  @rust-tui
  @bug-fix
  Scenario: Control characters are removed except newlines
    Given a bash command outputs text containing control characters (0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F)
    When the tool output is processed for TUI display
    Then all control characters are removed from the displayed text
    And newline characters are preserved so multi-line output renders correctly

  @unit
  @rust-tui
  @bug-fix
  Scenario: Plain text output passes through sanitization unchanged
    Given a bash command outputs plain text without any escape sequences or control characters
    When the tool output is processed for TUI display
    Then the text is displayed exactly as output by the command
    And no characters are removed or modified

  @unit
  @rust-tui
  @bug-fix
  Scenario: ToolProgress streaming chunks are also sanitized
    Given a bash command streams output line-by-line via ToolProgress
    When each streaming chunk is processed for TUI display
    Then ANSI escape sequences are stripped from each chunk before rendering
    And the user sees clean output in real-time as it streams

  @integration
  @rust-tui
  @bug-fix
  Scenario: LLM receives raw unsanitized output while TUI gets sanitized
    Given a bash command outputs text with ANSI color codes
    When the tool result is returned to both the LLM and the TUI
    Then the LLM receives the raw output with ANSI codes intact
    And the TUI scrollback contains only the sanitized plain text
