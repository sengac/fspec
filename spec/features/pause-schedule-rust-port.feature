@done
@schedule-management
@rust
@cli
@RPC-254
Feature: Port pause-schedule command to Rust
  """
  New impl file at rust/fspec-core/src/commands/pause_schedule.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` — the single source of truth shared by the LLM dispatcher AND the standalone fspec Rust binary (RPC-003 §7/§11 two-front-doors).
  Args struct: { name: String } with `#[serde(rename_all = "camelCase")]`. The on-disk model is SchedulesData { version: Option<String>, schedules: IndexMap<String, serde_json::Value>, #[serde(flatten)] extra } so unknown top-level fields round-trip and each schedule entry's unknown fields are preserved verbatim (parity with the TS `fileManager.transaction<SchedulesData>` round-trip).
  Behaviour parity with src/commands/schedule/pause-schedule.ts:23-43: validate the named schedule exists (else error "Schedule '<name>' does not exist"); validate it is not already paused (else error "Schedule '<name>' is already paused"); set its `status` to 'paused'; write spec/schedules.json atomically with JSON.stringify(...,2) semantics (2-space indent, NO trailing newline — file-manager.ts:361). Return `{ "success": true }`.
  DELIBERATE DIVERGENCE (flagged for supervisor): the TS code sets `data = {}` when the file is missing then crashes with a TypeError on `data.schedules[name]`. The Rust port models `schedules` with `#[serde(default)]` so a missing file deserializes to an empty map, yielding the cleaner "Schedule '<name>' does not exist" error instead of a crash.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for pause-schedule MUST replace the NotYetPorted stub
  #   2. Pausing sets status='paused' and writes spec/schedules.json atomically
  #   3. Missing schedule → error "Schedule '<name>' does not exist", no write
  #   4. Already paused → error "Schedule '<name>' is already paused", no write
  #   5. All other fields and other schedules preserved verbatim (flatten/extra round-trip)
  #   6. Both invocation paths converge on fspec_core::commands::pause_schedule::run
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to pause an active schedule by name
    So that I can temporarily stop a scheduled job from triggering without removing its configuration

  Scenario: Pause an active schedule sets its status to paused
    Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'active'
    When I dispatch the pause-schedule command with name='nightly-review' against that project root
    Then the dispatcher returns success=true
    And spec/schedules.json now records the 'nightly-review' schedule with status 'paused'

  Scenario: Pausing a missing schedule reports it does not exist
    Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'active'
    When I dispatch the pause-schedule command with name='ghost' against that project root
    Then the dispatcher returns an error with message "Schedule 'ghost' does not exist"
    And spec/schedules.json is unchanged

  Scenario: Pausing an already-paused schedule reports it is already paused
    Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'paused'
    When I dispatch the pause-schedule command with name='nightly-review' against that project root
    Then the dispatcher returns an error with message "Schedule 'nightly-review' is already paused"
    And spec/schedules.json is unchanged

  Scenario: Pausing one of several schedules preserves the others verbatim
    Given spec/schedules.json contains three schedules 'alpha', 'beta', and 'gamma' all with status 'active' and distinct cron, timezone, and jobType fields
    When I dispatch the pause-schedule command with name='beta' against that project root
    Then the dispatcher returns success=true
    And only the 'beta' schedule has status 'paused'
    And the 'alpha' and 'gamma' schedules retain their original status and all sibling fields verbatim
    And the 'beta' schedule retains its cron, timezone, and jobType fields verbatim
