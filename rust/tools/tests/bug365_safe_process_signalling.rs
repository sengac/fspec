//! BUG-365 — the tools crate signals processes through a safe API and
//! reports the result.
//!
//! Feature: spec/features/tools-safe-process-signalling.feature

#![cfg(unix)]

use codelet_tools::process_signal::{kill_process, kill_process_group, KillOutcome};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn killing_a_running_process_group_terminates_it() -> TestResult {
    // @step Given a running process started in its own process group
    let mut child = tokio::process::Command::new("sleep")
        .arg("30")
        .process_group(0)
        .kill_on_drop(true)
        .spawn()?;
    let pid = child.id().ok_or("no pid")?;
    // @step When its process group is killed
    let outcome = kill_process_group(pid)?;
    // @step Then the kill reports the group was signalled and the process exits
    assert_eq!(outcome, KillOutcome::Signalled);
    let status = codelet_test_helpers::deadlock_guard("child.wait()", child.wait()).await??;
    assert!(!status.success(), "{status:?}");
    Ok(())
}

#[tokio::test]
async fn signalling_a_process_that_already_exited_reports_it_was_gone() -> TestResult {
    // @step Given a process that has already exited and been reaped
    let mut child = tokio::process::Command::new("true").spawn()?;
    let pid = child.id().ok_or("no pid")?;
    child.wait().await?;
    // @step When it is killed
    let outcome = kill_process(pid)?;
    // @step Then the kill reports the process was already gone
    assert_eq!(outcome, KillOutcome::AlreadyGone);
    Ok(())
}

#[test]
fn the_tools_sources_contain_no_unsafe_code_suppressions() -> TestResult {
    // @step Given the tools crate sources
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![root];
    let mut offenders = Vec::new();
    // @step When they are scanned for unsafe code suppressions
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && std::fs::read_to_string(&path)?.contains("allow(unsafe_code)")
            {
                offenders.push(path.display().to_string());
            }
        }
    }
    // @step Then no source file allows unsafe_code
    assert!(offenders.is_empty(), "{offenders:?}");
    Ok(())
}
