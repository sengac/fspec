@done
@rust
@agent-view
@tui
@RPC-390
Feature: Port Edit/Write diff generation and [R]-/[A]+ marker encoding (Rust TUI)
  """
  Port targets: src/git/diff-parser.ts (computeLineDiff, changesToDiffLines) + src/tui/components/AgentView.tsx (formatEditDiff:623, formatWriteDiff:644, formatDiffForDisplay:670, formatWithTreeConnectors:551, calculateStartLine:781)
  New pure module rust/fspec-tui/src/store/agent_view/diff_format.rs; add similar='2' to fspec-tui Cargo.toml (workspace-pinned, already used by rust/git). No rendering/wire-up here (RPC-391).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An Edit diff is computed from (old_string, new_string) using a Myers line diff (similar crate)
  #   2. A Write diff treats the entire new file content as additions
  #   3. Removed lines encode as '{lineNum} [R]- {content}', added as '{lineNum} [A]+ {content}', context as '{lineNum}   {content}' with no marker
  #   4. Only changed lines plus 3 lines of surrounding context are shown; skipped regions collapse to a '... (N lines)' gap marker
  #   5. Output longer than DIFF_COLLAPSED_LINES (25) is truncated with a '... +N lines (select turn to /expand)' indicator
  #   6. Line numbers are offset by startLine and left-padded to at least width 3; tree connectors prefix the first line 'L ' and indent subsequent lines two spaces
  #   7. calculateStartLine returns the 1-based line of the edit, or 1 when the file or string is unavailable, and never panics
  #
  # EXAMPLES:
  #   1. Single-line replacement produces one [R]- line and one [A]+ line within 3-line context
  #   2. Pure addition (old_string empty) produces only [A]+ lines and no [R]- lines
  #   3. A Write of a 3-line file produces three [A]+ lines
  #   4. A 100-line edit with a single mid-file change shows leading and trailing '... (N lines)' gap markers
  #   5. A diff exceeding 25 display lines ends with a '... +N lines (select turn to /expand)' indicator
  #   6. calculateStartLine on a missing/unreadable file returns 1
  #   7. calculateStartLine finds new_string at line 250 of a file and returns 250
  #
  # ========================================
  Background: User Story
    As a fspec-tui developer
    I want to generate marker-encoded Edit/Write diffs from tool inputs in Rust
    So that the colored-diff rendering layer (RPC-391) has a tested, TS-faithful diff source

  Scenario: Single-line replacement produces one removed and one added marker within context
    Given an old_string and new_string that differ in a single line
    When I format the edit diff for display
    Then the output contains exactly one [R]- line and one [A]+ line
    And the surrounding context lines appear within three lines of the change

  Scenario: Pure addition produces only added markers and no removed markers
    Given an empty old_string and a new_string with several lines
    When I format the edit diff for display
    Then every change line is an [A]+ line
    And no [R]- line appears in the output

  Scenario: Write of a three-line file produces three added markers
    Given a Write content of exactly three lines
    When I format the write diff for display
    Then the output contains three [A]+ lines
    And no [R]- line appears in the output

  Scenario: A mid-file change in a large edit drops the leading region and shows a trailing gap marker
    Given a 100-line edit with a single changed line in the middle
    When I format the edit diff for display
    Then the leading context begins at the first shown line and earlier lines are dropped
    And a trailing '... (N lines)' gap marker follows the change context

  Scenario: A diff exceeding the collapse limit ends with an expand indicator
    Given a diff whose display lines exceed the collapse limit of 25
    When I format the edit diff for display
    Then the output is truncated to the first 25 display lines
    And the last line is '... +N lines (select turn to /expand)'

  Scenario: calculateStartLine on a missing file returns 1
    Given a file path that does not exist
    When I calculate the start line for the edit
    Then the start line is 1
    And no panic occurs

  Scenario: calculateStartLine finds new_string at line 250 and returns 250
    Given a file whose 250th line contains the new_string
    When I calculate the start line for the edit
    Then the start line is 250

  Scenario: Context lines are encoded with a line number and three spaces and no marker
    Given a diff with an unchanged context line
    When I format the edit diff for display
    Then the context line shows the line number followed by three spaces and the content
    And the context line carries no [R] or [A] marker

  Scenario: Line numbers are offset by startLine and left-padded to at least width three
    Given an edit positioned with a startLine of 250
    When I format the edit diff for display with that startLine
    Then the first marker line shows the offset line number 250
    And line numbers are left-padded to at least width three

  Scenario: Tree connectors prefix the first line and indent the rest while empty content yields empty
    Given a multi-line content string
    When I apply tree connectors to the content
    Then the first line is prefixed with 'L ' and subsequent lines are indented two spaces
    And empty or whitespace-only content yields an empty string

  Scenario: A representative edit produces a byte-for-byte golden display string
    Given a representative edit with a known old_string and new_string and startLine
    When I format the edit diff for display
    Then the output equals the expected golden string byte-for-byte

  Scenario: Trailing-newline and no-trailing-newline content produce parity diff lines
    Given an old_string and new_string that differ in one line, in both a trailing-newline and a no-trailing-newline variant
    When I format the edit diff for display for both variants
    Then both variants produce identical diff lines
    And the display output is identical byte-for-byte
