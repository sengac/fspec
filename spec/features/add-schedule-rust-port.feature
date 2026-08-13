@done
@feature-management
@cli
@rust
@RPC-191
Feature: Port add-schedule command to Rust
  """
  New impl at rust/fspec-core/src/commands/add_schedule.rs replaces the NotYetPorted stub. Signature: pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError>. Args struct (camelCase) mirrors AddScheduleOptions: name, cron, timezone, jobType, overlapPolicy?, role?, prompt?, command?
  SHARED-FILE REQUEST: cron validation. TS uses cron-validate npm pkg with 5-field preset (no seconds, no L/W/#/blank-day). Rust workspace already has croner=2 + chrono-tz=0.10 deps used by rust/core/src/scheduler/cron_utils.rs (parse_cron via croner::Cron::new(expr).parse(); parse_timezone via tz_str.parse::<Tz>()). Need supervisor to add `croner` and `chrono-tz` to rust/fspec-core/Cargo.toml [dependencies]. NOTE: croner default may accept 6-field/seconds; we enforce the 5-field count check first (matching TS split length==5) before croner parse to keep parity.
  schedules.json IO: TS ensureSchedulesFile auto-creates {version:'1.0.0',schedules:{}} via fileManager.readJSON. Rust will add a shared io helper ensure_schedules_file(project_root) -> SchedulesData (auto-create) + write_json_atomic for the write, OR inline read_or_init_json with default. PREFERENCE: add ensure_schedules_file to rust/fspec-core/src/io/ensure.rs — SHARED FILE, supervisor-owned. Will ASK supervisor to add it; remove-schedule (RPC-280) also needs the schedules path helper. Model SchedulesData as { version: String (default 1.0.0), schedules: IndexMap<String, serde_json::Value> } to preserve insertion order + round-trip unknown fields.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Rust dispatcher route for `add-schedule` MUST replace the NotYetPorted stub at rust/fspec-core/src/commands/add_schedule.rs
  #   2. Schedule name must be a lowercase hyphenated slug matching ^[a-z0-9]+(-[a-z0-9]+)*$ (after trim); otherwise error 'Invalid schedule name '<name>'. Names must be lowercase, hyphenated slugs (e.g., 'nightly-review', 'daily-sync').'
  #   3. Cron expression must be exactly 5 whitespace-separated fields; otherwise error 'Invalid cron expression: expected 5 fields (minute hour dayOfMonth month dayOfWeek), got <n>'. Then it must pass standard cron validation; an invalid expression errors with 'Invalid cron expression: <detail>'
  #   4. Timezone must be a valid IANA timezone string (trimmed); otherwise error 'Invalid timezone '<tz>'. ...' with suggestions when available
  #   5. jobType must be 'agent' or 'shell'; any other value errors 'Invalid jobType: <type>. Must be 'agent' or 'shell'.'
  #   6. agent jobType requires both role and prompt; missing either errors 'Agent schedules require both role and prompt'. shell jobType requires command; missing it errors 'Shell schedules require a command'
  #   7. spec/schedules.json is auto-created with default {version:'1.0.0', schedules:{}} if missing (parity with ensureSchedulesFile)
  #   8. Duplicate name: if data.schedules already has options.name, error 'Schedule '<name>' already exists' and no write occurs
  #   9. On success the new entry is written with fields in order: name, cron, timezone, overlapPolicy (default 'skip'), status 'active', lastRunAt null, lastRunStatus null, createdAt ISO8601, then jobType plus type-specific role/prompt (agent) or command (shell). Write is atomic
  #   10. Validation order matches TS: name → cron → timezone → jobType-specific fields → ensure file → duplicate check. Validation failures occur before any file write
  #   11. CLI bridge delegates to the single fspec_core source of truth; CLI flags mirror TS: -n/--name, -c/--cron, -z/--timezone, -t/--type, -r/--role, -p/--prompt, --command, -o/--overlap (default skip)
  #
  # EXAMPLES:
  #   1. Add agent schedule 'nightly-review' cron '0 2 * * *' tz UTC role 'Security reviewer' prompt 'Review src/' → success, entry written with jobType=agent, status=active
  #   2. Add shell schedule 'daily-tests' cron '30 6 * * 1-5' tz America/New_York command 'npm test' → success, entry written with jobType=shell, overlapPolicy=skip
  #   3. Add schedule when spec/schedules.json missing → file auto-created with version 1.0.0 then entry inserted
  #   4. Add schedule with name 'My Schedule' (spaces/uppercase) → error about lowercase hyphenated slugs, no write
  #   5. Add schedule with cron '0 2 * *' (4 fields) → error 'expected 5 fields ... got 4', no write
  #
  # ========================================
  Background: User Story
    As a developer using the standalone fspec Rust binary
    I want to dispatch add-schedule from the agent loop AND invoke `fspec add-schedule` from a shell
    So that I can register recurring agent or shell jobs in spec/schedules.json, sharing one source of truth between the LLM dispatcher and the CLI

  Scenario: Add an agent schedule writes the entry with status active
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'nightly-review' cron '0 2 * * *' timezone 'UTC' jobType 'agent' role 'Security reviewer' prompt 'Review src/'
    Then the dispatcher returns success=true
    And spec/schedules.json contains a schedule named 'nightly-review'
    And the 'nightly-review' entry has jobType='agent', status='active', cron='0 2 * * *', timezone='UTC'
    And the 'nightly-review' entry has role='Security reviewer' and prompt='Review src/'
    And the 'nightly-review' entry has overlapPolicy='skip', lastRunAt=null, lastRunStatus=null

  Scenario: Add a shell schedule writes the entry with default skip overlap policy
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'daily-tests' cron '30 6 * * 1-5' timezone 'America/New_York' jobType 'shell' command 'npm test'
    Then the dispatcher returns success=true
    And spec/schedules.json contains a schedule named 'daily-tests'
    And the 'daily-tests' entry has jobType='shell', overlapPolicy='skip', command='npm test'

  Scenario: spec/schedules.json is auto-created when missing
    Given a project root directory with no spec/schedules.json file
    When I dispatch the add-schedule command with name 'weekly-deps' cron '0 9 * * 1' timezone 'Europe/London' jobType 'shell' command 'npx depcheck'
    Then the dispatcher returns success=true
    And spec/schedules.json exists with version '1.0.0'
    And spec/schedules.json contains a schedule named 'weekly-deps'

  Scenario: Invalid schedule name is rejected without writing
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'My Schedule' cron '0 2 * * *' timezone 'UTC' jobType 'shell' command 'echo hi'
    Then the dispatcher returns an error mentioning lowercase hyphenated slugs
    And spec/schedules.json contains no schedule named 'My Schedule'

  Scenario: Cron expression with fewer than five fields is rejected without writing
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'bad-cron' cron '0 2 * *' timezone 'UTC' jobType 'shell' command 'echo hi'
    Then the dispatcher returns an error mentioning expected 5 fields, got 4
    And spec/schedules.json contains no schedule named 'bad-cron'

  Scenario: Invalid timezone is rejected without writing
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'bad-tz' cron '0 2 * * *' timezone 'Not/AZone' jobType 'shell' command 'echo hi'
    Then the dispatcher returns an error mentioning the invalid timezone
    And spec/schedules.json contains no schedule named 'bad-tz'

  Scenario: Invalid jobType is rejected without writing
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'bad-type' cron '0 2 * * *' timezone 'UTC' jobType 'webhook' command 'echo hi'
    Then the dispatcher returns an error mentioning jobType must be 'agent' or 'shell'
    And spec/schedules.json contains no schedule named 'bad-type'

  Scenario: Agent schedule missing role and prompt is rejected
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'incomplete-agent' cron '0 2 * * *' timezone 'UTC' jobType 'agent' with no role or prompt
    Then the dispatcher returns an error mentioning agent schedules require both role and prompt
    And spec/schedules.json contains no schedule named 'incomplete-agent'

  Scenario: Shell schedule missing command is rejected
    Given an empty project root directory
    When I dispatch the add-schedule command with name 'incomplete-shell' cron '0 2 * * *' timezone 'UTC' jobType 'shell' with no command
    Then the dispatcher returns an error mentioning shell schedules require a command
    And spec/schedules.json contains no schedule named 'incomplete-shell'

  Scenario: Duplicate schedule name is rejected and existing entry is preserved
    Given spec/schedules.json already contains a schedule named 'nightly-review'
    When I dispatch the add-schedule command with name 'nightly-review' cron '0 3 * * *' timezone 'UTC' jobType 'shell' command 'echo dup'
    Then the dispatcher returns an error mentioning the schedule already exists
    And the existing 'nightly-review' entry is unchanged
