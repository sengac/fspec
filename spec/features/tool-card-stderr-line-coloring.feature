@done
@agent-view
@rust
@tui
@RPC-400
Feature: Stderr lines in tool cards must render red and strip the stderr sentinel (TS parity)
  """
  Marker constant: define/reuse a single Rust STDERR_MARKER='⚠stderr⚠' in fspec-tui, locked by a parity test to codelet-tools bash_output.rs value. Prefer a small shared const rather than a cross-crate dependency if it keeps fspec-tui decoupled.
  Live path (chunk_processor::handle_tool_progress): when info.is_stderr, prefix each non-empty split('\n') line with STDERR_MARKER before appending, mirroring AgentView.tsx:2485-2490. is_stderr=false path unchanged.
  Scrollback render (chunk_wrap::wrap_tool_call non-diff body loop): per hard body line, compute a red flag = source.is_error(whole-card) OR line.contains(STDERR_MARKER); strip ALL marker occurrences from the line; style red when flag else body_style. Prefix/header untouched. Diff branch (is_diff) bypassed.
  Modal render (diff_decode::style_modal_lines non-diff branch): apply the same per-line marker strip + red styling so the TurnContentModal matches the scrollback. Whole-card is_error may not be plumbed into style_modal_lines today — thread the marker-based per-line detection at minimum; if whole-card error red is needed in modal, thread an is_error param.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A body line containing the STDERR_MARKER (⚠stderr⚠) is rendered red, with every marker occurrence stripped from the visible text
  #   2. When the whole tool card is an error (is_error=true), the entire body renders red and all stderr markers are stripped (existing whole-card error behavior preserved)
  #   3. Stderr lines are rendered red even when the command succeeded (is_error=false) — a mixed stdout/stderr successful command shows only the stderr lines red, stdout lines in normal body color
  #   4. On the live streaming path, a ToolProgress chunk with is_stderr=true has each non-empty line prefixed with the STDERR_MARKER when folded into the card body, so it renders red (parity with the settled path which already carries the marker)
  #   5. A ToolProgress chunk with is_stderr=false is folded verbatim (no marker added) and renders in the normal body color
  #   6. The STDERR_MARKER text never reaches the screen in any render path — it is stripped in both the scrollback tool-card body and the TurnContentModal
  #   7. Diff cards (is_diff=true) bypass stderr detection entirely and are unaffected
  #   8. The marker constant value is exactly ⚠stderr⚠, matching codelet/tools bash_output.rs STDERR_MARKER and the TS reference
  #
  # EXAMPLES:
  #   1. A settled tool card body line '⚠stderr⚠warning: unused import' renders as 'warning: unused import' in red (marker stripped)
  #   2. A successful command (is_error=false) whose body is 'Compiling main.rs' then '⚠stderr⚠warning: unused var' renders 'Compiling main.rs' in the normal color and 'warning: unused var' in red
  #   3. A failed command (is_error=true) whose body mixes stdout and '⚠stderr⚠error: cannot find value' renders the whole body red and no marker text is visible
  #   4. A live ToolProgress chunk 'error: boom\nmore' with is_stderr=true is folded so both lines become '⚠stderr⚠error: boom' and '⚠stderr⚠more' and both render red
  #   5. A live ToolProgress chunk 'Listening on :3000' with is_stderr=false is folded verbatim (no marker) and renders in the normal body color
  #   6. Opening the TurnContentModal (Enter) on a card with stderr lines shows those lines red with the marker stripped, matching the scrollback
  #
  # ========================================
  Background: User Story
    As a developer watching bash tool output in the Rust fspec-tui
    I want to see stderr lines rendered red with the ⚠stderr⚠ sentinel stripped, on both live streaming and settled cards
    So that I can visually distinguish diagnostic/error output from normal output exactly as the original TypeScript TUI did, without the marker text leaking to screen

  Scenario: Settled stderr line renders red with the marker stripped
    Given a settled tool card whose command succeeded
    And a body line "⚠stderr⚠warning: unused import"
    When the tool card body is rendered in the scrollback
    Then the line displays as "warning: unused import"
    And the line is styled red
    And no "⚠stderr⚠" marker text is visible

  Scenario: A successful command shows only its stderr lines red
    Given a settled tool card whose command succeeded
    And a body with line "Compiling main.rs" then line "⚠stderr⚠warning: unused var"
    When the tool card body is rendered in the scrollback
    Then the line "Compiling main.rs" is styled in the normal body color
    And the line "warning: unused var" is styled red
    And no "⚠stderr⚠" marker text is visible

  Scenario: A failed command renders the whole body red with no marker visible
    Given a settled tool card whose command failed
    And a body mixing stdout with line "⚠stderr⚠error: cannot find value"
    When the tool card body is rendered in the scrollback
    Then every body line is styled red
    And no "⚠stderr⚠" marker text is visible

  Scenario: Live stderr progress is prefixed with the marker so it renders red
    Given a streaming tool card
    When a ToolProgress chunk "error: boom\nmore" arrives with is_stderr true
    Then the card body gains the lines "⚠stderr⚠error: boom" and "⚠stderr⚠more"
    And both lines are styled red when rendered

  Scenario: Live non-stderr progress is folded verbatim in the normal color
    Given a streaming tool card
    When a ToolProgress chunk "Listening on :3000" arrives with is_stderr false
    Then the card body gains the line "Listening on :3000" with no marker added
    And the line is styled in the normal body color when rendered

  Scenario: The content modal shows stderr lines red with the marker stripped
    Given a settled tool card with a body line "⚠stderr⚠error: cannot find value"
    When the TurnContentModal is opened for that card
    Then the line displays as "error: cannot find value"
    And the line is styled red
    And no "⚠stderr⚠" marker text is visible

  Scenario: Diff cards bypass stderr detection entirely
    Given a settled tool card whose body is an Edit/Write diff
    When the tool card body is rendered in the scrollback
    Then stderr detection is not applied to the diff rows
    And the diff rows keep their existing removed/added/context styling
