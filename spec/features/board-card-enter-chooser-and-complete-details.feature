@BOARD-024
@tui
@dialog
@board
@integration
@rust
@phase-1
Feature: Board card Enter chooser and complete details
  """
  Overlay dialogs preserve the captured card and Board or Mux origin. Agent
  entry delegates to existing intent-safe EnterWorkUnit or MuxEnterWorkUnit.
  Canonical details read the complete stored record from the daemon workspace,
  not WorkUnitInfo, the show-work-unit projection or UI-local filesystem data.
  JSON travels as a string over both transports for bincode compatibility.
  Readable sections include userStory, descriptions, relationships, discovery,
  answers, stateHistory and legacy history, timestamps, blockedReason, eventStorm,
  hooks, iteration/token accounting, attachments and additional unknown fields.
  Deleted discovery items remain distinguishable as historical information.
  Request generations reject stale successes and errors, including same-ID reopen.
  Rendered text uses terminal sanitization; identifiers and paths retain original
  values for actions. Attachment viewing preserves existing lexical path rejection
  without claiming symlink containment. Metadata existence alone is insufficient:
  directories are not openable attachments. Failures retain the current details.
  Useful errors explain the failure category in addition to naming the target;
  merely displaying an attachment filename is not an error. Routing an attachment
  action is distinct from a successful browser launch. Stale hitbox rejection
  applies to in-bounds old coordinates after scrolling as well as offscreen rows.
  New implementation modules must stay below 300 lines. Production integration
  is paused pending the separately documented oversized-file/refactor decision.
  Shared dispatch branch ownership is cleared; BUG-283 navigator scope is excluded.
  """

  Background: User Story
    As a developer using the kanban board
    I want to choose agent entry or inspect complete card details on Enter
    So that I can inspect linked context without changing sessions

  Scenario: Board Enter opens a modal chooser and Escape preserves context
    Given a board with two cards and an unrelated open agent
    When the user presses Enter then navigation shortcut and paste keys
    Then the chooser offers Open in agent and Show full details without changing context
    And Escape closes only the chooser without creating or reassigning an agent

  Scenario: Default agent choice uses the captured card and earliest associated agent
    Given multiple associated agents and an unrelated focused agent
    When Enter opens the chooser and a board refresh selects a different card
    Then confirming the default choice opens the earliest agent for the captured card
    And no agent assignment or creation occurs

  Scenario: Unassociated agent choice retains normal isolated and cancel behavior
    Given a card without an associated agent and an unrelated open agent
    When the user chooses Open in agent from the card chooser
    Then the captured card offers normal isolated and cancel creation choices
    And only a confirmed creation attaches the captured card to its returned agent

  Scenario: Mux Enter overlays the chooser without replacing the grid
    Given a focused mux board pane and associated and unrelated open agents
    When Enter opens the chooser and Escape cancels before reopening and confirming
    Then cancellation preserves layout pane focus card and agent assignments
    And confirmation focuses the earliest associated agent inside the same grid

  Scenario: Empty board Enter does not offer card actions
    Given an empty focused board column
    When the user presses Enter
    Then no chooser or agent creation opens and the board remains unchanged

  Scenario: Full canonical details agree across embedded and WebSocket backends
    Given a daemon workspace with a complete rich card and a truncated board summary
    When the user chooses Show full details through each transport
    Then all stored sections including historical and unknown fields are reachable
    And canonical Unicode text is not replaced by the board summary

  Scenario: Missing and malformed canonical cards show errors without losing the chooser
    Given a board summary whose canonical store is missing malformed or invalid
    When the user chooses Show full details
    Then a visible useful error retains the captured card and origin
    And Escape returns through the chooser without creating an agent

  Scenario: Reopening the same card rejects stale success and error completions
    Given an earlier details request has completed but its response is delayed
    When the user closes and reopens the same card after its canonical data changes
    Then the current generation shows only the new canonical details
    And delivering an older success or error cannot replace the current details

  Scenario: Dismissed requests cannot resurrect dialogs or replace another card
    Given a completed details response has not yet reached the open dialog
    When the user dismisses it or opens a different card before response delivery
    Then the delayed response cannot reopen the dismissed dialog or replace the other card

  Scenario: Unicode details wrap scroll and retain focus on resize
    Given long Unicode details with a selectable linked card near the end
    When the user scrolls by arrows pages endpoints and wheel then resizes narrowly
    Then the first and final wrapped lines remain reachable without clipping
    And Tab reveals the selected link and resize preserves its activation target

  Scenario: Linked card cycles support back history and missing targets retain context
    Given details for linked cards A and B and a missing linked card
    When Tab and Enter follow A to B to A and Escape unwinds that history
    Then each parent restores its scroll and focused link before returning to the chooser
    And opening the missing link shows an error while the current details remain usable

  Scenario: Attachments retain exact paths and unavailable viewer failures retain details
    Given details with duplicate attachment basenames and Unicode spaces hash and percent paths
    When the user selects each attachment with Tab Shift-Tab and Enter
    Then the existing attachment action receives its exact original path
    And an unavailable viewer shows a visible error without closing details

  Scenario: Missing directory and rejected attachment paths show contextual errors
    Given details with missing directory and traversal attachment targets
    When the user activates each attachment
    Then each failure is visible and the current card and attachment selection remain usable

  Scenario: Display sanitization preserves original identifiers and attachment actions
    Given canonical text with terminal control sequences and an attachment path requiring encoding
    When the user renders details and selects the attachment
    Then rendered text contains no unsafe terminal controls
    And attachment activation retains the original path for existing URL encoding

  Scenario: Details consume modal shortcuts and paste without changing underlying state
    Given complete details are open above a board or mux with an unrelated agent
    When the user types shortcut keys modified arrows and a multiline paste
    Then the details stay open and board selection session input and assignments remain unchanged

  Scenario: Resized details use current mouse geometry and reveal keyboard focus
    Given details with linked cards below the initial viewport
    When Tab reveals a link and the terminal resizes before a mouse click
    Then only the latest rendered link hitbox activates its visible target
    And an old or offscreen hitbox cannot activate a stale link

  Scenario: Footer and help describe card choices and details controls
    Given the board and its help are rendered
    When the user opens the card chooser and full details
    Then hints describe the chooser and scrolling link attachment and back controls
