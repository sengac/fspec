@BOARD-024
@tui
@dialog
@board
@integration
@rust
@phase-1
Feature: Board card details scrolling and links
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
  builds on the REFAC-019 composed RPC, backend and Action surfaces.
  Shared dispatch branch ownership is cleared; BUG-283 navigator scope is excluded.
  """

  Background: User Story
    As a developer using the kanban board
    I want to choose agent entry or inspect complete card details on Enter
    So that I can inspect linked context without changing sessions

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
