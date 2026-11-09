@BUG-582
@done
@RPC-061
@rust
@tui
@rpc
@agent-view
@supervisor
@session-management
Feature: Supervisor / subordinate links — App + AgentView integration
  """
  Phase 7.8 of the RPC-030 roadmap. Wires the supervisor / subordinate
  link surface (WATCH-003/006/008/011/019/020) end-to-end through the
  Rust ratatui AgentView. Adds Action::SupervisorsLoaded /
  Action::SendToSubordinate, spawn_load_supervisors on
  Action::SessionCreated, the SupervisorPendingInjection chunk →
  store.supervisor_pending_count_for bridge, and the error-path
  EmitSessionNotice wiring. Out of scope: a `/send` slash-command UX
  entry point, auto-promotion of subordinates on supervisor
  disconnect.

  Companion features:
  - spec/features/rpc061-source-shape.feature
  - spec/features/rpc061-cross-transport-parity.feature
  TUI-132 (2026-11-09): Action::SendToSubordinate and handle_send_to_subordinate are removed with their scenarios: nothing emitted the action.
  """

  Background: User Story
    As a fspec TUI user with an open AgentView and (optionally) a supervisor session
    I want to see when my session is a subordinate, get notified when supervisors inject messages, and dispatch supervisor-to-subordinate messages from the Rust ratatui frontend
    So that I have full parity with the TS Ink supervisor/subordinate workflow without any NAPI dependency

  Scenario: App::dispatch routes Action::SupervisorsLoaded into the AgentViewStore
    Given an App wired to a MockBackend
    When Action::SupervisorsLoaded(SessionId("s-1"), vec![SessionId("sup")]) is dispatched
    Then store.supervisors_for(&SessionId("s-1")) returns &[SessionId("sup")]

  Scenario: StreamChunk::SupervisorPendingInjection bumps per-session pending count to 1
    Given an App with open session s-sub wired to a MockBackend
    And store.supervisor_pending_count_for(&SessionId("s-sub")) == 0
    When Action::ChunkReceived(s-sub, StreamChunk::SupervisorPendingInjection) is dispatched
    Then store.supervisor_pending_count_for(&SessionId("s-sub")) returns 1

  Scenario: Two consecutive SupervisorPendingInjection chunks bump count to 2
    Given an App with open session s-sub wired to a MockBackend
    When two Action::ChunkReceived events carrying SupervisorPendingInjection are dispatched
    Then store.supervisor_pending_count_for(&SessionId("s-sub")) returns 2

  Scenario: Action::SessionCreated triggers spawn_load_supervisors and SupervisorsLoaded
    Given an App wired to a MockBackend whose get_supervisors(SessionId("s-1")) returns vec![SessionId("sup")]
    When Action::SessionCreated(SessionId("s-1")) is dispatched
    Then within 1 second store.supervisors_for(&SessionId("s-1")) returns [SessionId("sup")]

  Scenario: A session with no supervisors shows no subordinate badge
    Given the AgentViewStore has no supervisors recorded for session s-sub
    When format_subordinate_label is called with an empty supervisors slice
    Then the helper returns None and the SessionHeader paints no [Subordinate of: ...] badge

  Scenario: Multi-supervisor session renders subordinate badge with +N count
    Given a session is recorded with three supervisors (s-sup-aaa, s-sup-bbb, s-sup-ccc)
    When format_subordinate_label is called with that supervisors slice
    Then the helper returns Some("s-sup-aa+2") (first 8 chars of the first supervisor id, plus +<count of remaining supervisors>)

  Scenario: Supervisor pending chip suppresses the compaction chip
    Given a SessionFooter is constructed with supervisor_pending_count=1 AND a CompactionProgress in flight
    When the footer is rendered to a buffer
    Then the left-aligned slot paints [1 pending from supervisor] in yellow
    And no [compacting: ...] chip is painted (the supervisor signal wins for that frame)

  Scenario: AttachToSession triggers spawn_load_supervisors and SupervisorsLoaded
    Given an App wired to a MockBackend whose get_supervisors(SessionId("s-sub")) returns vec![SessionId("sup")]
    When Action::AttachToSession(SessionId("s-sub")) is dispatched
    Then within 1 second store.supervisors_for(&SessionId("s-sub")) returns [SessionId("sup")]
