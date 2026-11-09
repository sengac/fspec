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
  builds on the REFAC-019 composed RPC, backend and Action surfaces.
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
