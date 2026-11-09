//! BUG-498 — schedule log appends hold the spec lock.
//!
//! Feature: spec/features/generated-spec-markdown-and-the-schedule-log-are-written-under-the-spec-lock.feature

use codelet_core::scheduler::job_log::{append_log_entry, JobLogEntry};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_schedule_log_appends_never_interleave() -> TestResult {
    // @step Given a project spec directory
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("spec"))?;
    let log = root.path().join("spec/schedule-log.jsonl");
    // @step When eight threads each append 300 schedule log entries at the same time
    let mut tasks = Vec::new();
    for writer in 0..8 {
        let log = log.clone();
        tasks.push(tokio::spawn(async move {
            for n in 0..300 {
                let entry = JobLogEntry {
                    timestamp: format!("2026-11-06T00:00:{n:02}Z"),
                    event: "triggered".to_string(),
                    schedule: format!("writer-{writer}"),
                    job_type: "shell".to_string(),
                    session_id: None,
                    duration_ms: None,
                    exit_code: None,
                    error: None,
                    message: Some(format!("entry {n}")),
                };
                append_log_entry(&log, &entry).await;
            }
        }));
    }
    for task in tasks {
        task.await?;
    }
    // @step Then the schedule log holds every entry since its one rotation, each a whole JSON line
    // 2400 appends: the append that finds 2001 lines rotates to the newest
    // 1000 first, so serialised appends leave 1000 + 1 + 398 = 1399 lines.
    let content = std::fs::read_to_string(&log)?;
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1399);
    for line in lines {
        serde_json::from_str::<serde_json::Value>(line)?;
    }
    Ok(())
}
