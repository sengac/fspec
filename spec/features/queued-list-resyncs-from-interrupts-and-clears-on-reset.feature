@BUG-582
@done
@TUI-119
@tui
@agent-view
Feature: Queued list resyncs from interrupts and clears on reset
  """
  The agent view's per-session queued list (rust/fspec-tui/src/store/agent_view/busy_inputs.rs, BusyInputs) is
  painted under the activity row while the session is busy (TUI-115). Before TUI-119 an entry left the list only
  when the session echoed the oldest queued text as StreamChunk::UserInput.
  A LIVE StreamChunk::Interrupted or InterruptedWithReason carries the session's authoritative queued_inputs (oldest
  first, BackgroundSession::queued_inputs). App::handle_live_transcript_wake (app/dispatch_transcript_live.rs) calls
  SessionContext::apply_live_interrupt_queue, which replaces the list with them: a listed entry matching the next
  session text in order keeps its number, entries the session no longer holds are dropped, and a text the TUI never
  listed is appended with the next number after the highest one listed. Transcript replay (record_chunk) never
  resyncs, because journal-replayed interrupts carry an empty queue (sessions/src/replay_transcript.rs) and may be
  historic.
  A /clear empties steers and queued entries (SessionContext::reset_for_clear), both for the local
  SlashCommandAction::Clear path (AgentView::reset_local_scrollback) and the session_cleared event
  (App::handle_session_cleared), so numbering restarts at 1. A transcript generation recovery
  (recover_transcript_generation) still calls plain reset_scrollback and keeps the list, because the session still
  holds those inputs. No wire change.
  """

  Background: User Story
    As a user steering and queueing input while a turn runs
    I want the queued list to match the inputs the session still holds
    So that an interrupt or a /clear never leaves stale queued entries on screen

  Scenario: Interrupted resyncs the queued list from the session's queued inputs
    Given the focused session's turn is running
    And the user queued "first", "second" and "third" with Alt+Enter
    When the session sends Interrupted with queued inputs "second" and "third"
    Then the queued list shows "Queued #2: second" and "Queued #3: third"
    And the queued list no longer shows "first"

  Scenario: InterruptedWithReason appends a queued input the agent view never listed
    Given the focused session's turn is running
    And the user queued "first" with Alt+Enter
    When the session sends InterruptedWithReason User with queued inputs "first" and "from another client"
    Then the queued list shows "Queued #1: first" and "Queued #2: from another client"
    When the user queues "next" with Alt+Enter
    Then the transcript shows "Queued #3: next"

  Scenario: A local /clear empties the queued list and restarts its numbering
    Given the focused session's turn is running
    And the user queued "stale follow-up" with Alt+Enter
    When the user runs /clear
    Then the queued list is empty
    When the user queues "fresh follow-up" with Alt+Enter
    Then the queued list shows "Queued #1: fresh follow-up"

  Scenario: The session_cleared event empties the queued list
    Given the focused session's turn is running
    And the user queued "stale follow-up" with Alt+Enter
    When the session_cleared event arrives for the focused session
    Then the queued list is empty

  Scenario: A queued input after a resync is numbered past the highest listed number
    Given the focused session's turn is running
    And the user queued "first" with Alt+Enter
    When the session sends Interrupted with queued inputs "from another client" and "first"
    Then the queued list shows "Queued #2: from another client" and "Queued #1: first"
    When the user queues "next" with Alt+Enter
    Then the queued list shows "Queued #3: next" last

  Scenario: A transcript-replayed interrupt leaves the queued list alone
    Given the focused session's turn is running
    And the user queued "first" with Alt+Enter
    When the transcript replays an interrupt that carries no queued inputs
    And the live Interrupted carries queued input "first"
    Then the queued list shows "Queued #1: first"
