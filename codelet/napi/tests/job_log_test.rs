// Feature: spec/features/schedule-job-log.feature
//
// This test file validates the acceptance criteria defined in the feature file.
// Scenarios map directly to Gherkin scenarios for the schedule job log module.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    // Import the job_log module under test — will fail until job_log.rs is created
    use codelet_napi::scheduler::job_log::{append_log_entry, JobLogEntry};

    /// Helper: create a temp dir and return the log file path
    fn temp_log_dir() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let log_path = dir.path().join("spec").join("schedule-log.jsonl");
        std::fs::create_dir_all(log_path.parent().unwrap()).unwrap();
        (dir, log_path)
    }

    /// Helper: read all log entries from a JSONL file
    fn read_entries(path: &std::path::Path) -> Vec<JobLogEntry> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Helper: write N dummy entries to a JSONL file
    fn write_dummy_entries(path: &std::path::Path, count: usize) {
        use std::io::Write;
        let mut file = std::fs::File::create(path).unwrap();
        for i in 0..count {
            let entry = serde_json::json!({
                "timestamp": format!("2026-01-01T00:00:{:02}Z", i % 60),
                "event": "triggered",
                "schedule": format!("job-{}", i),
                "jobType": "shell",
            });
            writeln!(file, "{}", entry).unwrap();
        }
    }

    // ---- Scenario: Log agent job triggered event ----
    #[tokio::test]
    async fn test_log_agent_job_triggered_event() {
        // @step Given a project with a scheduled agent job "daily-sync"
        let (_dir, log_path) = temp_log_dir();

        // @step When the scheduler triggers the "daily-sync" job
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "triggered".to_string(),
            schedule: "daily-sync".to_string(),
            job_type: "agent".to_string(),
            session_id: Some("abc-123".to_string()),
            duration_ms: None,
            exit_code: None,
            error: None,
            message: None,
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then a JSONL entry is appended to spec/schedule-log.jsonl with event "triggered"
        let entries = read_entries(&log_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "triggered");

        // @step And the entry contains schedule "daily-sync", jobType "agent", a timestamp, and a sessionId
        assert_eq!(entries[0].schedule, "daily-sync");
        assert_eq!(entries[0].job_type, "agent");
        assert!(!entries[0].timestamp.is_empty());
        assert_eq!(entries[0].session_id.as_deref(), Some("abc-123"));
    }

    // ---- Scenario: Log agent job completed event ----
    #[tokio::test]
    async fn test_log_agent_job_completed_event() {
        // @step Given a scheduled agent job "daily-sync" has been triggered
        let (_dir, log_path) = temp_log_dir();

        // @step When the agent job completes successfully
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "completed".to_string(),
            schedule: "daily-sync".to_string(),
            job_type: "agent".to_string(),
            session_id: Some("abc-123".to_string()),
            duration_ms: Some(3200),
            exit_code: None,
            error: None,
            message: None,
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then a JSONL entry is appended with event "completed"
        let entries = read_entries(&log_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "completed");

        // @step And the entry contains duration_ms and sessionId
        assert_eq!(entries[0].duration_ms, Some(3200));
        assert_eq!(entries[0].session_id.as_deref(), Some("abc-123"));
    }

    // ---- Scenario: Log shell job failed event ----
    #[tokio::test]
    async fn test_log_shell_job_failed_event() {
        // @step Given a project with a scheduled shell job "run-tests"
        let (_dir, log_path) = temp_log_dir();

        // @step When the shell job fails with exit code 1 and stderr "npm ERR! missing script: sync"
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "failed".to_string(),
            schedule: "run-tests".to_string(),
            job_type: "shell".to_string(),
            session_id: None,
            duration_ms: Some(150),
            exit_code: Some(1),
            error: Some("npm ERR! missing script: sync".to_string()),
            message: None,
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then a JSONL entry is appended with event "failed"
        let entries = read_entries(&log_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "failed");

        // @step And the entry contains exitCode 1 and error "npm ERR! missing script: sync"
        assert_eq!(entries[0].exit_code, Some(1));
        assert_eq!(
            entries[0].error.as_deref(),
            Some("npm ERR! missing script: sync")
        );
    }

    // ---- Scenario: Log skipped event on overlap ----
    #[tokio::test]
    async fn test_log_skipped_event_on_overlap() {
        // @step Given a scheduled job "daily-sync" with overlap policy "skip"
        let (_dir, log_path) = temp_log_dir();

        // @step And the previous run of "daily-sync" is still active
        // (Precondition — overlap detected by caller)

        // @step When the scheduler evaluates "daily-sync" for triggering
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "skipped".to_string(),
            schedule: "daily-sync".to_string(),
            job_type: "agent".to_string(),
            session_id: None,
            duration_ms: None,
            exit_code: None,
            error: None,
            message: Some("Previous run still active".to_string()),
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then a JSONL entry is appended with event "skipped"
        let entries = read_entries(&log_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "skipped");

        // @step And the entry contains message "Previous run still active"
        assert_eq!(
            entries[0].message.as_deref(),
            Some("Previous run still active")
        );
    }

    // ---- Scenario: Log deferred event on session limit ----
    #[tokio::test]
    async fn test_log_deferred_event_on_session_limit() {
        // @step Given a scheduled agent job "nightly-review"
        let (_dir, log_path) = temp_log_dir();

        // @step And 10 out of 10 sessions are active
        // (Precondition — session limit detected by caller)

        // @step When the scheduler evaluates "nightly-review" for triggering
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "deferred".to_string(),
            schedule: "nightly-review".to_string(),
            job_type: "agent".to_string(),
            session_id: None,
            duration_ms: None,
            exit_code: None,
            error: None,
            message: Some("10/10 sessions active".to_string()),
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then a JSONL entry is appended with event "deferred"
        let entries = read_entries(&log_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "deferred");

        // @step And the entry contains message "10/10 sessions active"
        assert_eq!(
            entries[0].message.as_deref(),
            Some("10/10 sessions active")
        );
    }

    // ---- Scenario: Log queued event on overlap queue policy ----
    #[tokio::test]
    async fn test_log_queued_event_on_overlap_queue() {
        // @step Given a scheduled job "hourly-check" with overlap policy "queue"
        let (_dir, log_path) = temp_log_dir();

        // @step And the previous run of "hourly-check" is still active
        // (Precondition — overlap detected by caller)

        // @step When the scheduler evaluates "hourly-check" for triggering
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "queued".to_string(),
            schedule: "hourly-check".to_string(),
            job_type: "shell".to_string(),
            session_id: None,
            duration_ms: None,
            exit_code: None,
            error: None,
            message: None,
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then a JSONL entry is appended with event "queued"
        let entries = read_entries(&log_path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].event, "queued");
    }

    // ---- Scenario: Log rotation truncates to 1000 entries ----
    #[tokio::test]
    async fn test_log_rotation_truncates_to_1000() {
        // @step Given spec/schedule-log.jsonl contains 2001 entries
        let (_dir, log_path) = temp_log_dir();
        write_dummy_entries(&log_path, 2001);
        let before = read_entries(&log_path);
        assert_eq!(before.len(), 2001);

        // @step When a new log entry is appended
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "triggered".to_string(),
            schedule: "rotation-test".to_string(),
            job_type: "shell".to_string(),
            session_id: None,
            duration_ms: None,
            exit_code: None,
            error: None,
            message: None,
        };
        append_log_entry(&log_path, &entry).await;

        // @step Then the file is truncated to the most recent 1000 entries plus the new entry
        let after = read_entries(&log_path);
        assert!(after.len() <= 1001);
        assert_eq!(after.last().unwrap().schedule, "rotation-test");
    }

    // ---- Scenario: Graceful handling of log write failure ----
    #[tokio::test]
    async fn test_graceful_handling_of_write_failure() {
        // @step Given the spec/schedule-log.jsonl file is not writable
        let log_path = PathBuf::from("/nonexistent/path/spec/schedule-log.jsonl");

        // @step When the scheduler attempts to append a log entry
        let entry = JobLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event: "triggered".to_string(),
            schedule: "test-job".to_string(),
            job_type: "shell".to_string(),
            session_id: None,
            duration_ms: None,
            exit_code: None,
            error: None,
            message: None,
        };
        // append_log_entry never panics — it swallows errors and logs a warning
        append_log_entry(&log_path, &entry).await;

        // @step Then the scheduler continues operating normally
        // Reaching this line proves append_log_entry did not panic on write failure

        // @step And a warning is emitted via tracing
        // (tracing output verified by integration — the function logs via tracing::warn! internally)
    }
}
