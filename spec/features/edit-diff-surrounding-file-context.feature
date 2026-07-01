@done
@diff-display
@RPC-394
@rust
@agent-view
@tui
Feature: Inject surrounding file context lines into Edit diffs

  """
  Architecture: extend PendingDiffKind::Edit to carry file_path; produce_diff_strings threads file_path into a new context-aware edit-diff builder in diff_format.rs (or a small helper module) that reads the post-edit file, slices CONTEXT_LINES before/after the changed span, and prepends/appends Context DiffOutputLines before running build_diff_rows. Keep files <300 LoC.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An Edit diff reads the post-edit file and injects up to CONTEXT_LINES (3) real unchanged file lines before and after the changed region as Context rows
  #   2. When the file is missing or unreadable, or the new_string/old_string cannot be located, the diff falls back to fragments-only with no injected context and never panics
  #   3. An Edit near the top of the file shows no before-context (clamped) but still shows after-context; an Edit near the bottom shows before-context but no after-context
  #   4. Write diffs (new file) are unaffected: the whole content remains additions with no injected context
  #   5. Injected before/after context lines render as Context rows (gray gutter, white content, no background) with correct 1-based file line numbers, preserving RPC-392/393 styling, and the existing windowing/elision/collapse logic runs over the merged sequence
  #   6. Injected context lines must not duplicate lines already present inside the changed region (no double-printing of unchanged lines)
  #
  # EXAMPLES:
  #   1. An Edit replacing two fully-different lines in the middle of a 50-line file shows the 3 unchanged lines above and 3 unchanged lines below as gray context, the old lines on red, the new lines on green
  #   2. An Edit changing line 1 of a file shows zero before-context rows and up to 3 after-context rows
  #   3. An Edit changing the last line of a file shows up to 3 before-context rows and zero after-context rows
  #   4. An Edit whose file_path is None or unreadable produces the same fragments-only diff as today with no panic and no context rows
  #   5. A Write of a brand-new 3-line file still shows three green added lines and no context rows
  #
  # ========================================

  Background: User Story
    As a fspec-tui user
    I want to see a few unchanged file lines above and below an Edit's changed lines
    So that I can read each change in its surrounding file context, not just isolated red/green lines

  Scenario: A mid-file edit shows three unchanged lines above and below the change
    Given a fifty-line file whose lines ten and eleven are replaced by two entirely different lines
    When I build the context-aware edit diff rows for that edit
    Then three unchanged file lines immediately above the change appear as gray context rows
    And the two old lines appear as removed rows and the two new lines appear as added rows
    And three unchanged file lines immediately below the change appear as gray context rows

  Scenario: An edit on the first line shows no before-context and trailing context
    Given a file whose first line is replaced by a different line
    When I build the context-aware edit diff rows for that edit
    Then no context row appears before the change
    And up to three unchanged file lines below the change appear as gray context rows

  Scenario: An edit on the last line shows leading context and no after-context
    Given a file whose last line is replaced by a different line
    When I build the context-aware edit diff rows for that edit
    Then up to three unchanged file lines above the change appear as gray context rows
    And no context row appears after the change

  Scenario: A missing or unreadable file falls back to fragments-only with no panic
    Given an edit whose file path does not exist on disk
    When I build the context-aware edit diff rows for that edit
    Then the rows contain only the removed and added fragment lines with no injected context rows
    And no panic occurs

  Scenario: A Write of a new file shows only added lines and no context rows
    Given a Write of a brand-new three-line file
    When I build the diff rows for that write
    Then the output contains three added rows
    And no context row appears in the output

  Scenario: A shared boundary line is shown once and never duplicated by injected context
    Given an edit whose old and new strings share an unchanged middle line inside a larger file
    When I build the context-aware edit diff rows for that edit
    Then the shared line appears exactly once as a gray context row
    And no injected after-context row duplicates the shared line
