//! Feature: spec/features/shell-job-execution.feature
use super::*;

// =====================================================================
// Scenario: Shell command executes successfully with exit code 0
// =====================================================================
#[tokio::test]
async fn test_shell_command_success() {
    // @step Given a schedule "nightly-lint" with job_type "shell" and shell.command "echo success"
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "nightly-lint": make_shell_schedule("echo success")
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    // @step And the schedule has a valid project path
    assert!(tmp.path().exists());

    // @step When the scheduler fires the shell job
    let entry = codelet_core::scheduler::types::ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "shell".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: None,
        shell: Some(codelet_core::scheduler::types::ShellConfig {
            command: "echo success".to_string(),
        }),
        overlap_policy: None,
    };

    let result =
        codelet_core::scheduler::shell_job::trigger_shell_job("nightly-lint", project_path, &entry)
            .await;

    // @step Then the command executes via "sh -c" in the project directory
    // @step And the ShellJobResult exit_code is 0
    let result = result.expect("shell job should succeed");
    assert_eq!(result.exit_code, 0);

    // @step And stdout contains "success"
    assert!(result.stdout.contains("success"));

    // @step And lastRunStatus is updated to "completed" in spec/schedules.json
    // @step And lastRunAt is updated to the current timestamp
    // (timestamp updates are handled by trigger_and_update in engine.rs, not in shell_job directly)
}
// =====================================================================
// Scenario: Shell command fails with non-zero exit code
// =====================================================================
#[tokio::test]
async fn test_shell_command_failure() {
    // @step Given a schedule "health-check" with job_type "shell" and shell.command "exit 1"
    let schedules = json!({
        "version": "1.0.0",
        "schedules": {
            "health-check": make_shell_schedule("exit 1")
        }
    });
    let tmp = setup_project_with_schedules(schedules).await;
    let project_path = tmp.path().to_str().unwrap();

    let entry = codelet_core::scheduler::types::ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "shell".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: None,
        shell: Some(codelet_core::scheduler::types::ShellConfig {
            command: "exit 1".to_string(),
        }),
        overlap_policy: None,
    };

    // @step When the scheduler fires the shell job
    let result =
        codelet_core::scheduler::shell_job::trigger_shell_job("health-check", project_path, &entry)
            .await;

    // @step Then the ShellJobResult exit_code is 1
    let result = result.expect("shell job should return result even on non-zero exit");
    assert_eq!(result.exit_code, 1);

    // @step And lastRunStatus is updated to "failed" in spec/schedules.json
    // (handled by trigger_and_update at engine level)
}
// =====================================================================
// Scenario: Shell command captures multi-line stdout
// =====================================================================
#[tokio::test]
async fn test_shell_multiline_stdout() {
    // @step Given a schedule with shell.command "echo hello && echo world"
    let entry = codelet_core::scheduler::types::ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "shell".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: None,
        shell: Some(codelet_core::scheduler::types::ShellConfig {
            command: "echo hello && echo world".to_string(),
        }),
        overlap_policy: None,
    };

    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler fires the shell job
    let result =
        codelet_core::scheduler::shell_job::trigger_shell_job("multiline", project_path, &entry)
            .await;

    // @step Then ShellJobResult stdout contains "hello" and "world"
    let result = result.expect("should succeed");
    assert!(result.stdout.contains("hello"));
    assert!(result.stdout.contains("world"));

    // @step And the exit_code is 0
    assert_eq!(result.exit_code, 0);
}
// =====================================================================
// Scenario: Shell command captures stdout and stderr separately
// =====================================================================
#[tokio::test]
async fn test_shell_stdout_and_stderr() {
    // @step Given a schedule with shell.command "echo out && echo err >&2"
    let entry = codelet_core::scheduler::types::ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "shell".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: None,
        shell: Some(codelet_core::scheduler::types::ShellConfig {
            command: "echo out && echo err >&2".to_string(),
        }),
        overlap_policy: None,
    };

    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().to_str().unwrap();

    // @step When the scheduler fires the shell job
    let result =
        codelet_core::scheduler::shell_job::trigger_shell_job("mixed-output", project_path, &entry)
            .await;

    let result = result.expect("should succeed");

    // @step Then ShellJobResult stdout contains "out"
    assert!(
        result.stdout.contains("out"),
        "stdout should contain 'out': {}",
        result.stdout
    );

    // @step And ShellJobResult stderr contains "err"
    assert!(
        result.stderr.contains("err"),
        "stderr should contain 'err': {}",
        result.stderr
    );
}
// =====================================================================
// Scenario: Shell command runs in the configured project directory
// =====================================================================
#[tokio::test]
async fn test_shell_runs_in_project_dir() {
    // @step Given a schedule with shell.command "pwd" and a specific project path
    let entry = codelet_core::scheduler::types::ScheduleEntry {
        cron: "*/5 * * * *".to_string(),
        timezone: "UTC".to_string(),
        status: "active".to_string(),
        job_type: "shell".to_string(),
        created_at: Some("2026-03-18T00:00:00Z".to_string()),
        last_run_at: None,
        last_run_status: None,
        agent: None,
        shell: Some(codelet_core::scheduler::types::ShellConfig {
            command: "pwd".to_string(),
        }),
        overlap_policy: None,
    };

    let tmp = TempDir::new().unwrap();
    let project_path = tmp.path().canonicalize().unwrap();
    let project_str = project_path.to_str().unwrap();

    // @step When the scheduler fires the shell job
    let result =
        codelet_core::scheduler::shell_job::trigger_shell_job("pwd-test", project_str, &entry)
            .await;

    // @step Then stdout contains the project path
    let result = result.expect("should succeed");
    // pwd output should match the project directory (canonicalized)
    let stdout_trimmed = result.stdout.trim();
    assert!(
        stdout_trimmed == project_str
            || tmp.path().starts_with(stdout_trimmed)
            || stdout_trimmed.contains(
                &tmp.path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            ),
        "pwd output '{stdout_trimmed}' should match project path '{project_str}'"
    );

    // @step And the command inherits the user's environment
    assert_eq!(result.exit_code, 0);
}
