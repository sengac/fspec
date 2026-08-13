@done
@SCHED-005
@scheduling
@napi
@rust
Feature: Shell Job Execution
  """
  Create rust/napi/src/scheduler/shell_job.rs with trigger_shell_job function using tokio::process::Command
  Add ShellJobResult struct to types.rs: exit_code, stdout, stderr
  Wire shell job routing in engine.rs evaluate_and_run — match on job_type field to choose agent vs shell
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Shell jobs execute via tokio::process::Command with sh -c on the configured command string
  #   2. Shell jobs run in the project directory (cwd = project path from the schedule)
  #   3. Shell jobs inherit the user's environment (PATH, etc.)
  #   4. Exit code 0 means success (lastRunStatus: completed), non-zero means failure (lastRunStatus: failed)
  #   5. stdout and stderr are captured from the command execution
  #   6. Shell jobs have no execution timeout — the command runs to completion
  #   7. After completion, lastRunAt timestamp and lastRunStatus are updated in spec/schedules.json
  #   8. Shell jobs require a non-empty shell.command field in the schedule config
  #   9. Shell jobs with missing or empty shell config fail gracefully with error status
  #
  # EXAMPLES:
  #   1. Schedule 'nightly-lint' has shell config with command='npm run lint'. Command runs in project dir, exits 0. lastRunStatus updated to 'completed', stdout captured.
  #   2. Schedule 'health-check' has shell config with command='curl http://localhost:3000/health'. Command exits non-zero (connection refused). lastRunStatus set to 'failed', stderr captured.
  #   3. Schedule 'bad-shell' has shell config with empty command string. trigger_shell_job returns error immediately without spawning a process.
  #   4. Schedule 'no-shell-config' has no shell block at all (shell: null/missing). Job fails gracefully with error before attempting execution.
  #   5. Shell command 'echo hello && echo world' runs — both stdout lines captured in ShellJobResult.stdout.
  #   6. Shell command writes to both stdout and stderr. ShellJobResult captures them separately as distinct fields.
  #   7. After shell job completes, spec/schedules.json is re-read, lastRunAt is updated to current ISO timestamp, lastRunStatus reflects the outcome.
  #   8. Engine receives schedule with job_type='shell' and routes to trigger_shell_job instead of trigger_agent_job.
  #
  # ========================================
  Background: User Story
    As a system operator
    I want to have scheduled shell commands automatically execute when a schedule fires
    So that I can run automated maintenance tasks like builds, linting, and health checks on a cron schedule

  Scenario: Shell command executes successfully with exit code 0
    Given a schedule "nightly-lint" with job_type "shell" and shell.command "echo success"
    And the schedule has a valid project path
    When the scheduler fires the shell job
    Then the command executes via "sh -c" in the project directory
    And the ShellJobResult exit_code is 0
    And stdout contains "success"
    And lastRunStatus is updated to "completed" in spec/schedules.json
    And lastRunAt is updated to the current timestamp

  Scenario: Shell command fails with non-zero exit code
    Given a schedule "health-check" with job_type "shell" and shell.command "exit 1"
    When the scheduler fires the shell job
    Then the ShellJobResult exit_code is 1
    And lastRunStatus is updated to "failed" in spec/schedules.json

  Scenario: Shell job fails when command string is empty
    Given a schedule "bad-shell" with job_type "shell" and shell.command ""
    When the scheduler fires the shell job
    Then trigger_shell_job returns an error immediately
    And no child process is spawned
    And lastRunStatus is updated to "failed"

  Scenario: Shell job fails when shell config is missing entirely
    Given a schedule "no-shell-config" with job_type "shell" and no shell config block
    When the scheduler fires the shell job
    Then trigger_shell_job returns an error about missing shell configuration
    And lastRunStatus is updated to "failed"

  Scenario: Shell command captures multi-line stdout
    Given a schedule with shell.command "echo hello && echo world"
    When the scheduler fires the shell job
    Then ShellJobResult stdout contains "hello" and "world"
    And the exit_code is 0

  Scenario: Shell command captures stdout and stderr separately
    Given a schedule with shell.command "echo out && echo err >&2"
    When the scheduler fires the shell job
    Then ShellJobResult stdout contains "out"
    And ShellJobResult stderr contains "err"

  Scenario: Shell job updates schedules.json timestamps after completion
    Given a schedule "timed-job" with a previously recorded lastRunAt
    When the scheduler fires the shell job and it completes
    Then lastRunAt in spec/schedules.json is updated to a newer ISO timestamp
    And lastRunStatus reflects the actual exit code outcome

  Scenario: Engine routes shell job_type to trigger_shell_job
    Given a schedule with job_type "shell"
    When evaluate_and_run processes this schedule
    Then it calls trigger_shell_job instead of trigger_agent_job

  Scenario: Shell command runs in the configured project directory
    Given a schedule with shell.command "pwd" and a specific project path
    When the scheduler fires the shell job
    Then stdout contains the project path
    And the command inherits the user's environment

  Scenario: Shell job with missing shell.command field fails gracefully
    Given a schedule with shell config that has no "command" field
    When the scheduler fires the shell job
    Then trigger_shell_job returns an error about missing command
    And lastRunStatus is updated to "failed"
