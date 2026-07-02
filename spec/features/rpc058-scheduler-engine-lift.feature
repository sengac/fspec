@done
@RPC-058
@rust
@source-shape
@lift
@scheduler
@schedule-management
Feature: Scheduler engine lift into codelet-core::scheduler
  """
  Phase 7.5 of the RPC-030 roadmap. The scheduler engine modules
  currently in codelet/napi/src/scheduler/ (engine, state, cron_utils,
  types, trigger, agent_job, shell_job, catch_up, job_log) MUST be
  lifted into codelet/core/src/scheduler/ following the same lift
  pattern used by RPC-031..RPC-034 for the persistence layer.

  After the lift:

  - codelet/core/src/scheduler/mod.rs hosts all engine modules and a
  SchedulerHooks trait that replaces the direct
  crate::session_bindings::SessionManager::instance() calls. Engine
  APIs (spawn_scheduler, evaluate_and_run) take an
  `Arc<dyn SchedulerHooks>` so the engine can call into either NAPI
  or codelet-sessions without importing them.
  - codelet/napi/src/scheduler/mod.rs collapses to `pub use
  codelet_core::scheduler::{...}` plus the still-NAPI-resident
  `pub mod loop_store;` (RPC-059 lifts loop_store separately).
  - The pure CRUD helpers (validate_cron, validate_timezone,
  validate_add_request, with_schedules_lock, read_schedules_file,
  write_schedules_file, handle_add/list/pause/resume/remove) move
  out of codelet/napi/src/schedule_handler.rs into a new
  codelet/core/src/scheduler/crud.rs (NAPI-free). The remaining
  schedule_handler.rs becomes a thin shim that adapts
  ScheduleRequest/ScheduleResult.

  These tests pin the file layout so a future refactor cannot
  accidentally re-introduce the NAPI dependency.
  """

  Background: User Story
    As a developer of fspec
    I want source-shape tests to pin the scheduler-engine lift
    So that no future refactor can re-introduce a NAPI dependency into the engine or the CRUD layer

  Scenario: Scheduler engine modules live under codelet/core/src/scheduler/
    Given the directory codelet/core/src/scheduler/ exists
    Then it contains a file named "mod.rs"
    And it contains a file named "engine.rs"
    And it contains a file named "state.rs"
    And it contains a file named "cron_utils.rs"
    And it contains a file named "types.rs"
    And it contains a file named "trigger.rs"
    And it contains a file named "agent_job.rs"
    And it contains a file named "shell_job.rs"
    And it contains a file named "catch_up.rs"
    And it contains a file named "job_log.rs"
    And it contains a file named "crud.rs"

  Scenario: codelet-core declares a SchedulerHooks trait
    Given the file codelet/core/src/scheduler/mod.rs is compiled
    Then it declares a public trait named "SchedulerHooks"
    And SchedulerHooks declares a method named "get_session_count" returning usize
    And SchedulerHooks declares a method named "get_live_session_ids" returning Vec<Uuid>
    And SchedulerHooks declares a method named "spawn_scheduled_session" returning Result<(), String>
    And SchedulerHooks declares a method named "default_model" returning String

  Scenario: spawn_scheduler accepts an Arc<dyn SchedulerHooks>
    Given the file codelet/core/src/scheduler/engine.rs is compiled
    Then it declares a public fn named "spawn_scheduler" whose last parameter has type "Arc<dyn SchedulerHooks>"

  Scenario: The lifted engine has no crate::session_bindings references
    Given the directory codelet/core/src/scheduler/ exists
    Then no file under codelet/core/src/scheduler/ contains the text "crate::session_bindings"
    And no file under codelet/core/src/scheduler/ contains the text "session_bindings::SessionManager"

  Scenario: codelet/napi/src/scheduler/mod.rs is a thin re-export shim
    Given the file codelet/napi/src/scheduler/mod.rs is compiled
    Then it contains a "pub use codelet_core::scheduler" re-export statement
    And it still declares a public module named "loop_store"

  Scenario: The pure CRUD helpers live in codelet-core::scheduler::crud
    Given the file codelet/core/src/scheduler/crud.rs is compiled
    Then it declares a public fn named "schedule_add"
    And it declares a public fn named "schedule_list"
    And it declares a public fn named "schedule_pause"
    And it declares a public fn named "schedule_resume"
    And it declares a public fn named "schedule_remove"
    And the file does not contain the text "use napi"

  Scenario: codelet-sessions handle_impl wires the five new methods to crud.rs
    Given the file codelet/sessions/src/handle_impl.rs is compiled
    Then it implements "schedule_add" by delegating to codelet_core::scheduler::crud
    And it implements "schedule_list" by delegating to codelet_core::scheduler::crud
    And it implements "schedule_pause" by delegating to codelet_core::scheduler::crud
    And it implements "schedule_resume" by delegating to codelet_core::scheduler::crud
    And it implements "schedule_remove" by delegating to codelet_core::scheduler::crud

  Scenario: Lifted scheduler engine and cron_utils use captured-identifier format args (RPC-058 retro 2026-05-27)
    Given the codelet workspace inherits the lint level `-D warnings` which includes `clippy::uninlined_format_args`
    When I scan engine.rs and cron_utils.rs for `format!` / `anyhow!` / `panic!` / `println!` / `eprintln!` / `write!` / `writeln!` macro invocations that interpolate a bare identifier
    Then every such invocation uses the captured-identifier syntax `"... {name} ..."` instead of the trailing-argument syntax `"... {} ...", name`
    Given codelet/core/src/scheduler/engine.rs and codelet/core/src/scheduler/cron_utils.rs live at their post-RPC-058 location
    Then `cargo clippy -p codelet-sessions -- -D warnings` exits 0 with no `clippy::uninlined_format_args` errors against engine.rs:284, engine.rs:299, cron_utils.rs:41, or cron_utils.rs:49
