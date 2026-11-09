@BOARD-024
@tui
@dialog
@board
@integration
@rust
@phase-1
Feature: Board card details loading generations
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
