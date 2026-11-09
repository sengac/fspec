@done
@TUI-115
@tui
@agent-view
Feature: Running composer stays visible with steer and queue hints and tags
  """
  While the focused session is Running or Compacting, the agent view paints the activity row (spinner and status)
  ABOVE the composer instead of in its place, then a hint line under it. The composer stays editable, so a draft is
  never edited blind. Today rust/fspec-tui/src/views/agent/input_transition.rs
  (paint_input_or_spinner_with_waiting) replaces the MultiLineInput with the spinner row; input_area_height
  (views/agent/input_area.rs) must size activity row + composer rows + hint row while busy.
  Keys while Running: Enter submits FspecBackend::submit_input(session, text, InputMode::Steer); Alt+Enter submits
  InputMode::Queue (RIG-016, views/agent/queue_key.rs). While Compacting both Enter and Alt+Enter submit
  InputMode::Queue, because steering into compaction makes no sense. No backend or wire change: BackgroundSession
  already steers into or queues behind a Running or Compacting turn.
  Each busy submission is echoed in the transcript tagged "You (steer): <text>" or "Queued #<n>: <text>", with n
  counting the session's queued inputs in submission order. Queued inputs are listed under the activity row until
  the session starts them (the session leaves the busy turn that queued them and starts a turn with that input).
  Decisions: spec/attachments/TUI-115/interview-decisions.md. Follow-ups: TUI-116, TUI-117.
  """

  Background: User Story
    As a user typing while the agent works
    I want to see my draft and know what Enter and Alt+Enter will do
    So that I can steer or queue input without editing blind or guessing

  Scenario: The composer stays visible under the activity row while a turn runs
    Given the focused session's turn is running
    When the user types "also check the logs"
    Then the activity row is painted above the composer
    And the composer shows "also check the logs"
    And the hint line under the composer reads "Enter steer · Alt+Enter queue · Esc interrupt"

  Scenario: Enter while a turn runs steers the input and tags it as a steer
    Given the focused session's turn is running and the composer holds "also check the logs"
    When the user presses Enter
    Then the input is submitted to the session with mode Steer
    And the transcript shows "You (steer): also check the logs"
    And the composer is empty and still visible

  Scenario: Alt+Enter while a turn runs queues the input and lists it until it starts
    Given the focused session's turn is running and the composer holds "then run the tests"
    When the user presses Alt+Enter
    Then the input is submitted to the session with mode Queue
    And the transcript shows "Queued #1: then run the tests"
    And the queued list under the activity row shows "Queued #1: then run the tests"
    When the session starts a turn for "then run the tests"
    Then the queued list no longer shows "then run the tests"

  Scenario: Queued inputs are numbered in submission order
    Given the focused session's turn is running
    When the user queues "first follow-up" and then "second follow-up" with Alt+Enter
    Then the queued list shows "Queued #1: first follow-up" above "Queued #2: second follow-up"

  Scenario: The composer stays editable during compaction with a queue-only hint
    Given the focused session is compacting
    When the user types "then update the changelog"
    Then the activity row is painted above the composer
    And the composer shows "then update the changelog"
    And the hint line under the composer reads "Enter or Alt+Enter queue · runs after compaction"

  Scenario: Enter during compaction queues the input
    Given the focused session is compacting and the composer holds "then update the changelog"
    When the user presses Enter
    Then the input is submitted to the session with mode Queue
    And the transcript shows "Queued #1: then update the changelog"

  Scenario: An idle session shows the plain composer without a busy hint
    Given the focused session is idle
    When the user types "hello" and presses Enter
    Then no activity row or hint line was painted around the composer
    And "hello" is sent as a new turn and shown without a steer or queue tag
