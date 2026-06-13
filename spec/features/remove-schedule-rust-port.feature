@done
@feature-management
@cli
@rust
@RPC-280
Feature: Port remove-schedule command to Rust

  """
  New impl at codelet/fspec-core/src/commands/remove_schedule.rs replaces NotYetPorted stub. Signature pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>. Args struct (camelCase) { name: String }. Load spec/schedules.json (read existing, default empty if missing — TS does NOT ensure-create), check key, remove, write_json_atomic. SHARED-FILE REQUEST: reuse the same ensure_schedules_file/schedules path helper requested for RPC-191; remove only needs the path + read (not auto-create). Model SchedulesData { version, schedules: IndexMap<String, Value> } with #[serde(flatten)] extra. Returns {success:true} JSON.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `remove-schedule` MUST replace the NotYetPorted stub at codelet/fspec-core/src/commands/remove_schedule.rs
  #   2. Takes a single positional argument `name` (the schedule slug to remove)
  #   3. Uses getSchedulesFilePath (spec/schedules.json) — does NOT call ensureSchedulesFile; opens the file directly via fileManager.transaction. If the file is missing the transaction read still yields the default empty schedules map so the not-found branch fires
  #   4. If data.schedules does NOT contain `name`, error 'Schedule '<name>' does not exist' and no write occurs
  #   5. On success the entry is deleted from data.schedules and the file is atomically written; other schedules and their insertion order are preserved; returns success=true
  #   6. CLI bridge delegates to the single fspec_core source of truth; CLI takes positional <name> (no flags). Success prints '✓ Schedule '<name>' removed successfully'
  #
  # EXAMPLES:
  #   1. Remove existing schedule 'nightly-review' from a file containing two schedules → success, entry deleted, the other schedule remains
  #   2. Remove schedule 'does-not-exist' when not present → error 'Schedule 'does-not-exist' does not exist', file unchanged
  #   3. Remove from a file with three schedules ZED, AAA, MID → remove AAA, remaining order ZED, MID preserved (insertion order kept)
  #
  # ========================================

  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch remove-schedule from the agent loop AND invoke `fspec remove-schedule <name>` from a shell
    So that I can permanently delete an obsolete schedule from spec/schedules.json, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Remove an existing schedule deletes only that entry
    Given spec/schedules.json contains schedules named 'nightly-review' and 'daily-tests'
    When I dispatch the remove-schedule command with name 'nightly-review'
    Then the dispatcher returns success=true
    And spec/schedules.json contains no schedule named 'nightly-review'
    And spec/schedules.json still contains a schedule named 'daily-tests'

  Scenario: Removing a non-existent schedule returns an error and leaves the file unchanged
    Given spec/schedules.json contains a schedule named 'daily-tests'
    When I dispatch the remove-schedule command with name 'does-not-exist'
    Then the dispatcher returns an error mentioning the schedule does not exist
    And spec/schedules.json still contains a schedule named 'daily-tests'

  Scenario: Removing a schedule preserves the insertion order of the remaining schedules
    Given spec/schedules.json contains schedules declared in order ZED, AAA, MID
    When I dispatch the remove-schedule command with name 'AAA'
    Then the dispatcher returns success=true
    And the remaining schedules are in order ZED, MID
