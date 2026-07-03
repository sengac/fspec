@done
@clipboard
@mouse-events
@integration
@rust
@text-selection
@tui
@COPY-010
Feature: Text selection anchors at line-start instead of the pressed column

  """
  Root cause: the four view-level Begin handlers (scrollback_copy.rs:84-94, multiline_input_select.rs:120-137, turn_modal_select.rs:101-112, details_select.rs:91-104) hard-code anchor.col=0 and cursor.col=content_width; Extend only moves the cursor, so the anchor stays at column 0. The recognizer (gesture.rs) and region model (selection.rs) already carry the correct (row,col).
  Recommended fix: add SelectionGesture::BeginLine(Cell) emitted by SelectionRecognizer::tick (long-press); drag keeps emitting Begin+Extend. Each Begin handler sets anchor=cursor=press cell (precise); each BeginLine handler sets anchor col0/cursor content_width (whole line). Scrollback needs Action::SelectionBeginLine routed to a new ScrollbackList::selection_begin_line; the other three surfaces hold Selection locally. See spec/attachments/COPY-010/bug-analysis-anchor-at-pressed-column.md.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A drag selection's start cell is the cell where the left button was pressed (its real row AND column), not column 0
  #   2. Dragging then releasing copies exactly the cells from the pressed cell to the released cell; the first row of the selection starts at the pressed column
  #   3. A stationary long-press (held past the threshold) with no drag still selects and copies the WHOLE line under the press
  #   4. A zero-width drag (press and release on the same cell with no movement) copies nothing
  #   5. The precise-anchor and preserved long-press behavior applies identically to all four surfaces: scrollback, input composer, turn-content modal, and board details strip
  #
  # EXAMPLES:
  #   1. In the scrollback, a row reads "Hello world"; the user presses at column 6 and drags to column 11 and releases; the clipboard receives "world" (not "Hello world")
  #   2. In the input composer, a wrapped row reads "the quick brown fox"; the user presses at the "b" of "brown" and drags to the end of "fox" and releases; the clipboard receives "brown fox"
  #   3. In the turn-content modal, the user presses mid-line and drags right; the copied text starts at the pressed column, not the start of the line
  #   4. In the board details strip, the user presses mid-title and drags to the end of the title; the copied text starts at the pressed column
  #   5. The user long-presses (holds ~0.5s without moving) on a scrollback line and releases; the whole line is selected and copied (unchanged behavior)
  #   6. The user presses and releases on the same cell without moving (zero-width drag); nothing is copied
  #
  # ========================================

  Background: User Story
    As a user selecting transcript or input text with the mouse
    I want to have my drag selection begin exactly at the cell where I press the mouse
    So that I can copy a precise substring instead of always getting the whole line from its start

  Scenario: Dragging from a mid-line column in the scrollback copies from that column
    Given a scrollback whose visible row reads "Hello world" with mouse capture enabled
    When I press the left mouse button at column 6 of that row and drag to the end of the row and release
    Then the clipboard receives "world"
    And the copied text does not start at the beginning of the line

  Scenario: Dragging from a mid-word column in the input composer copies from that column
    Given a composer whose visible row reads "the quick brown fox"
    When I press the left mouse button at the start of "brown" and drag to the end of "fox" and release
    Then the clipboard receives "brown fox"

  Scenario: Dragging from a mid-line column in the turn-content modal copies from that column
    Given an open turn-content modal showing a body line with known text and mouse capture enabled
    When I press the left mouse button at a mid-line column of the body and drag to the end of the line and release
    Then the copied text starts at the pressed column and not at the start of the line

  Scenario: Dragging from a mid-title column in the board details strip copies from that column
    Given a board with a work unit selected and its details strip visible
    When I press the left mouse button at a mid-title column of the id and title row and drag to the end of the title and release
    Then the copied text starts at the pressed column and not at the start of the line

  Scenario: A long-press with no drag still selects and copies the whole line
    Given a scrollback whose visible row reads "whole line text" with mouse capture enabled
    When I press and hold the left mouse button on that row for about half a second without moving and release
    Then the whole line text is written to the clipboard
    And the selection stays highlighted

  Scenario: A zero-width drag copies nothing
    Given a scrollback whose visible row reads "Hello world" with mouse capture enabled
    When I press and release the left mouse button on the same cell without moving
    Then nothing is written to the clipboard

