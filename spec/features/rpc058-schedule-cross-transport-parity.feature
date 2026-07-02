@done
@tui
@RPC-058
@rpc
@rust
@parity
@schedule-management
Feature: /schedule cross-transport parity
  """
  Both EmbeddedFspecBackend (in-process embedded transport) and
  WebSocketFspecBackend (tarpc over WebSocket) must land identically on
  the same StubSessionManagerHandle for every new RPC method introduced
  by RPC-058:

  * schedule_add
  * schedule_list
  * schedule_pause
  * schedule_resume
  * schedule_remove

  Mirrors the RPC-049 / RPC-050 / RPC-054 / RPC-055 / RPC-056 / RPC-057
  cross-transport parity tests — each transport invocation increments
  the same per-stub counter and returns the same payload.
  """

  Background: User Story
    As a developer porting the AgentView to Rust
    I want both transports to land identically on the SessionManagerHandle for the /schedule RPCs
    So that the WebSocket and embedded paths cannot diverge as the feature grows

  Scenario: Embedded and WebSocket schedule_add both reach the stub
    Given a StubSessionManagerHandle seeded with a ScheduledJob { name: "daily", cron: "0 9 * * *", timezone: "UTC", job_type: "agent", status: "active", role: Some("reviewer"), prompt: Some("daily standup"), command: None, overlap_policy: Some("skip") } behind both an EmbeddedFspecBackend and a WebSocketFspecBackend
    When schedule_add is called via the embedded transport with name "daily" and cron "0 9 * * *" and timezone "UTC" and job_type "agent" and role Some("reviewer") and prompt Some("daily standup") and command None and overlap_policy Some("skip")
    And schedule_add is called via the WebSocket transport with name "daily" and cron "0 9 * * *" and timezone "UTC" and job_type "agent" and role Some("reviewer") and prompt Some("daily standup") and command None and overlap_policy Some("skip")
    Then the stub's schedule_add_calls counter equals 2
    And both calls return Ok(ScheduledJob) with byte-identical field values

  Scenario: Embedded and WebSocket schedule_list both reach the stub
    Given a StubSessionManagerHandle seeded with two ScheduledJob rows behind both transports
    When schedule_list is called via the embedded transport
    And schedule_list is called via the WebSocket transport
    Then the stub's schedule_list_calls counter equals 2
    And both calls return a Vec of length 2
    And each entry has identical name, cron, timezone, job_type, status, role, prompt, command, overlap_policy fields across the two transports

  Scenario: Embedded and WebSocket schedule_pause both reach the stub
    Given a StubSessionManagerHandle seeded with a ScheduledJob whose status is "paused" behind both transports
    When schedule_pause is called via the embedded transport with name "daily"
    And schedule_pause is called via the WebSocket transport with name "daily"
    Then the stub's schedule_pause_calls counter equals 2
    And both calls return Ok(ScheduledJob) with status equal to "paused"

  Scenario: Embedded and WebSocket schedule_resume both reach the stub
    Given a StubSessionManagerHandle seeded with a ScheduledJob whose status is "active" behind both transports
    When schedule_resume is called via the embedded transport with name "daily"
    And schedule_resume is called via the WebSocket transport with name "daily"
    Then the stub's schedule_resume_calls counter equals 2
    And both calls return Ok(ScheduledJob) with status equal to "active"

  Scenario: Embedded and WebSocket schedule_remove both reach the stub
    Given a StubSessionManagerHandle seeded to return Ok(()) for schedule_remove behind both transports
    When schedule_remove is called via the embedded transport with name "daily"
    And schedule_remove is called via the WebSocket transport with name "daily"
    Then the stub's schedule_remove_calls counter equals 2
    And both calls return Ok(())
