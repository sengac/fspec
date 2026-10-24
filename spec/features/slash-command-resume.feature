@done
@slash-command
@tui
@agent-view
@rpc
@multi-session
@rust
@session-management
@RPC-049
@BUG-244
Feature: Resume durable session state and hydrate the transcript
  """
  Resume installs authoritative model messages and saved token state. Transcript
  hydration uses bounded cursor pages independently of that model installation.
  Repeated attachment focuses the existing tab and continues its transcript
  cursor. Failures remain scoped to the originating session.
  """

  Background: User Story
    As a fspec user with a stored session on disk
    I want to select that session from the resume picker
    So that I can inspect its original conversation and continue the work

  Scenario: AttachToSession spawns backend.resume_session for the selected session
    Given an App wired to a MockBackend with sessions ["s-1", "s-2"]
    And ResumeSessionView is open with row 0 (s-1) selected
    When the user presses Enter on the resume view
    Then within 1 second backend.resume_session is called exactly once with session_id s-1
    And the AgentViewStore's open_sessions contains s-1
    And the AgentViewStore's current_session is s-1

  Scenario: Successful resume hydrates the complete transcript snapshot
    Given an App wired to a MockBackend whose resume_session returns Ok(())
    And the MockBackend's buffered_output for s-1 is [StreamChunk::text("hello"), StreamChunk::text("world")]
    When Action::AttachToSession(s-1) is dispatched
    Then within 1 second backend.resume_session is called exactly once with session_id s-1
    And bounded transcript pages are requested for session s-1 until the snapshot completes
    And within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "hello"
    And within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "world"

  Scenario: Failed resume exposes an incomplete transcript notice
    Given an App wired to a MockBackend whose resume_session returns Err("corrupt manifest")
    When Action::AttachToSession(s-1) is dispatched
    And periodic transcript polling and live notifications occur before another resume succeeds
    Then the originating session contains an incomplete transcript notice with the corrupt manifest reason
    And no transcript pages are requested before resume succeeds

  Scenario: Idempotent re-restore — AttachToSession on an already-open session still calls resume_session
    Given an App with open_sessions [s-1] and current_session s-1
    And the MockBackend's resume_session returns Ok(())
    When Action::AttachToSession(s-1) is dispatched
    Then within 1 second backend.resume_session is called exactly once with session_id s-1
    And open_sessions length stays 1 (no duplicate append)
