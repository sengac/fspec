@BUG-582
@BOARD-024
@tui
@dialog
@board
@integration
@rust
@phase-1
Feature: Board card details attachments
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
