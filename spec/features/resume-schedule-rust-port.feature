@done
@schedule-management
@rust
@cli
@RPC-292
Feature: Port resume-schedule command to Rust

  """
  New impl file at codelet/fspec-core/src/commands/resume_schedule.rs replaces the NotYetPorted stub. The module exposes `pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>` — the single source of truth shared by the LLM dispatcher AND the standalone fspec Rust binary (RPC-003 §7/§11 two-front-doors). Twin of pause-schedule (RPC-254); differs only in target status ('active') and the already-X guard message.
  Args struct: { name: String } with `#[serde(rename_all = "camelCase")]`. The on-disk model is SchedulesData { version: Option<String>, schedules: IndexMap<String, serde_json::Value>, #[serde(flatten)] extra } so unknown top-level fields round-trip and each schedule entry's unknown fields are preserved verbatim (parity with the TS `fileManager.transaction<SchedulesData>` round-trip).
  Behaviour parity with src/commands/schedule/pause-schedule.ts:51-71 (resumeSchedule): validate the named schedule exists (else error "Schedule '<name>' does not exist"); validate it is not already active (else error "Schedule '<name>' is already active"); set its `status` to 'active'; write spec/schedules.json atomically with JSON.stringify(...,2) semantics (2-space indent, NO trailing newline). Return `{ "success": true }`.
  DELIBERATE DIVERGENCE (flagged for supervisor): a missing spec/schedules.json deserializes to an empty schedules map (`#[serde(default)]`), yielding the cleaner "Schedule '<name>' does not exist" error rather than the TS TypeError crash.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for resume-schedule MUST replace the NotYetPorted stub
  #   2. Resuming sets status='active' and writes spec/schedules.json atomically
  #   3. Missing schedule → error "Schedule '<name>' does not exist", no write
  #   4. Already active → error "Schedule '<name>' is already active", no write
  #   5. All other fields and other schedules preserved verbatim (flatten/extra round-trip)
  #   6. Both invocation paths converge on fspec_core::commands::resume_schedule::run
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to resume a paused schedule by name
    So that I can re-enable a previously paused scheduled job so it triggers again on its cron expression

  Scenario: Resume a paused schedule sets its status to active
    Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'paused'
    When I dispatch the resume-schedule command with name='nightly-review' against that project root
    Then the dispatcher returns success=true
    And spec/schedules.json now records the 'nightly-review' schedule with status 'active'

  Scenario: Resuming a missing schedule reports it does not exist
    Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'paused'
    When I dispatch the resume-schedule command with name='ghost' against that project root
    Then the dispatcher returns an error with message "Schedule 'ghost' does not exist"
    And spec/schedules.json is unchanged

  Scenario: Resuming an already-active schedule reports it is already active
    Given spec/schedules.json contains a shell schedule named 'nightly-review' with status 'active'
    When I dispatch the resume-schedule command with name='nightly-review' against that project root
    Then the dispatcher returns an error with message "Schedule 'nightly-review' is already active"
    And spec/schedules.json is unchanged

  Scenario: Resuming one of several schedules preserves the others verbatim
    Given spec/schedules.json contains three schedules 'alpha' (paused), 'beta' (paused), and 'gamma' (active) with distinct cron, timezone, and jobType fields
    When I dispatch the resume-schedule command with name='beta' against that project root
    Then the dispatcher returns success=true
    And only the 'beta' schedule has status 'active'
    And the 'alpha' and 'gamma' schedules retain their original status and all sibling fields verbatim
    And the 'beta' schedule retains its cron, timezone, and jobType fields verbatim
