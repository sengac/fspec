@done
@tui
@RPC-058
@rpc
@rust
@source-shape
@schedule-management
Feature: /schedule RPC surface source shape
  """
  Pin the source-shape contract that the rest of RPC-030 depends on:

  - The five schedule_add/schedule_list/schedule_pause/schedule_resume/
  schedule_remove RPC methods MUST be declared at every layer of the
  dual-transport stack (SessionManagerHandle trait + tarpc
  FspecService + FspecBackend trait + both transport forwarders).
  - The new wire type ScheduledJob MUST exist as a public declaration
  in codelet-rpc-types with all twelve documented fields.
  - The ScheduleSubcommand enum + parse_schedule_command MUST live in
  codelet/fspec-tui/src/app/schedule_parser.rs.
  - All slash-command wiring for /schedule MUST live in
  codelet/fspec-tui/src/app/dispatch_slash_schedule.rs (mirrors
  dispatch_merge_worktree) so the orchestrator dispatch.rs stays under the
  300-LoC ceiling.

  These tests run against source files at compile/parse time — they
  catch refactors that accidentally collapse the dual-transport
  layering, drop any of the five RPC methods, or inline the /schedule
  wiring back into dispatch.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. One new wire type lives in codelet-rpc-types: ScheduledJob struct (flat, napi(object)-compatible).
  #   2. SessionManagerHandle MUST expose default-impl methods for all five RPC operations.
  #   3. StubSessionManagerHandle MUST expose per-call counters for cross-transport parity tests.
  #   4. FspecService (tarpc) MUST declare async fns for all five operations.
  #   5. FspecBackend trait MUST expose async variants of all five methods.
  #   6. EmbeddedFspecBackend AND WebSocketFspecBackend MUST forward each method to the tarpc client.
  #   7. ScheduleSubcommand enum + parse_schedule_command MUST live in codelet/fspec-tui/src/app/schedule_parser.rs.
  #   8. All /schedule wiring MUST live in codelet/fspec-tui/src/app/dispatch_slash_schedule.rs.
  #
  # ========================================
  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the layering of the /schedule RPC surface
    So that no future refactor can collapse the dual-transport boundary or strand the parser

  Scenario: ScheduledJob wire type is exported from codelet-rpc-types
    Given the file codelet/rpc-types/src/lib.rs is compiled
    Then it declares a public struct named "ScheduledJob"
    And ScheduledJob has fields named name, cron, timezone, job_type, status
    And ScheduledJob has fields named created_at, last_run_at, last_run_status
    And ScheduledJob has fields named role, prompt, command, overlap_policy

  Scenario: SessionManagerHandle declares the five new schedule methods
    Given the file codelet/core/src/session_manager_handle.rs is compiled
    Then it declares a trait method named "schedule_add" returning Result<ScheduledJob, String>
    And it declares a trait method named "schedule_list" returning Vec<ScheduledJob>
    And it declares a trait method named "schedule_pause" returning Result<ScheduledJob, String>
    And it declares a trait method named "schedule_resume" returning Result<ScheduledJob, String>
    And it declares a trait method named "schedule_remove" returning Result<(), String>

  Scenario: StubSessionManagerHandle exposes per-call counters for all five schedule methods
    Given the file codelet/core/src/session_manager_handle.rs is compiled
    Then StubSessionManagerHandle declares a method named "schedule_add_calls" returning u64
    And StubSessionManagerHandle declares a method named "schedule_list_calls" returning u64
    And StubSessionManagerHandle declares a method named "schedule_pause_calls" returning u64
    And StubSessionManagerHandle declares a method named "schedule_resume_calls" returning u64
    And StubSessionManagerHandle declares a method named "schedule_remove_calls" returning u64

  Scenario: FspecService declares the five new RPC methods
    Given the file codelet/rpc/src/lib.rs is compiled
    Then it declares an async fn named "schedule_add" with return type Result<ScheduledJob, String>
    And it declares an async fn named "schedule_list" with return type Vec<ScheduledJob>
    And it declares an async fn named "schedule_pause" with return type Result<ScheduledJob, String>
    And it declares an async fn named "schedule_resume" with return type Result<ScheduledJob, String>
    And it declares an async fn named "schedule_remove" with return type Result<(), String>

  Scenario: FspecBackend declares the five new methods
    Given the file codelet/fspec-tui/src/transport/mod.rs is compiled
    Then it declares an async fn named "schedule_add" on the FspecBackend trait returning Result<ScheduledJob>
    And it declares an async fn named "schedule_list" on the FspecBackend trait returning Result<Vec<ScheduledJob>>
    And it declares an async fn named "schedule_pause" on the FspecBackend trait returning Result<ScheduledJob>
    And it declares an async fn named "schedule_resume" on the FspecBackend trait returning Result<ScheduledJob>
    And it declares an async fn named "schedule_remove" on the FspecBackend trait returning Result<()>

  Scenario: Both transports implement the five new methods
    Given the files codelet/fspec-tui/src/transport/embedded.rs and codelet/fspec-tui/src/transport/websocket.rs are compiled
    Then each file contains an impl of "schedule_add" that calls the corresponding tarpc client method
    And each file contains an impl of "schedule_list" that calls the corresponding tarpc client method
    And each file contains an impl of "schedule_pause" that calls the corresponding tarpc client method
    And each file contains an impl of "schedule_resume" that calls the corresponding tarpc client method
    And each file contains an impl of "schedule_remove" that calls the corresponding tarpc client method

  Scenario: schedule_parser module exists with the documented entry points
    Given the file codelet/fspec-tui/src/app/schedule_parser.rs exists
    Then it declares a public enum named "ScheduleSubcommand"
    And ScheduleSubcommand has variants named Add, List, Pause, Resume, Remove, Help
    And it declares a public fn named "parse_schedule_command" taking &str and returning ScheduleSubcommand

  Scenario: /schedule slash command wiring lives in dispatch_slash_schedule.rs
    Given the file codelet/fspec-tui/src/app/dispatch_slash_schedule.rs exists
    Then it declares a method named "handle_slash_schedule_help"
    And it declares a method named "handle_schedule_subcommand"
    And it declares a method named "handle_schedule_add"
    And it declares a method named "handle_schedule_list"
    And it declares a method named "handle_schedule_pause"
    And it declares a method named "handle_schedule_resume"
    And it declares a method named "handle_schedule_remove"
    And it declares a method named "try_dispatch_slash_schedule"
