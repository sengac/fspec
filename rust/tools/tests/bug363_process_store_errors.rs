//! BUG-363 — the unified-exec process store reports child kill and try_wait
//! failures instead of discarding them.
//!
//! Feature: spec/features/unified-exec-process-errors-reported.feature

use codelet_tools::unified_exec::{
    global_store, UnifiedExecArgs, UnifiedExecResult, UnifiedExecTool,
};
use rig::tool::Tool;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The process-store sources, whitespace removed.
fn store_sources() -> Result<Vec<(String, String)>, std::io::Error> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/unified_exec");
    [
        "process_store.rs",
        "process_store_handles.rs",
        "process_store_eviction.rs",
    ]
    .iter()
    .map(|name| {
        let text = std::fs::read_to_string(dir.join(name))?;
        Ok((name.to_string(), text.split_whitespace().collect()))
    })
    .collect()
}

#[test]
fn process_store_kill_and_wait_failures_are_never_discarded() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // @step Given the unified exec process store sources
    let sources = store_sources()?;
    // @step When their child kill and try_wait calls are listed
    for (name, compact) in &sources {
        // @step Then no kill result is discarded
        assert!(
            !compact.contains("let_=entry.child.kill()")
                && !compact.contains("let_=e.child.kill()"),
            "{name} discards a child kill result"
        );
        // @step And no try_wait error is read as a running or exited process
        assert!(
            !compact.contains("Err(_)=>Some(None)"),
            "{name} reads a try_wait error as running"
        );
        assert!(
            !compact.contains(".is_some()).unwrap_or(false)"),
            "{name} reads a try_wait error as not exited"
        );
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn try_wait_distinguishes_a_running_session_from_an_unknown_one() -> TestResult {
    let isolated = codelet_test_helpers::isolated_user_directory();
    assert!(
        isolated.is_ok(),
        "BUG-640: isolate the user directory: {isolated:?}"
    );
    // BUG-447: this test drives tools without a session manager.
    codelet_tools::facade::declare_unrestricted_tool_access(
        "tests drive tools without a session manager",
    );
    // @step Given a unified exec session whose command is still running
    let tool = UnifiedExecTool::new(uuid::Uuid::new_v4());
    let started: UnifiedExecResult = tool
        .call(UnifiedExecArgs(serde_json::json!({
            "action": "run", "command": "sleep 5", "yield_time_ms": 50
        })))
        .await?;
    let session = started
        .session_id
        .ok_or("a running command keeps a session")?;
    // @step When the store checks the session with try_wait
    let running = global_store().try_wait(&session).await?;
    // @step Then it reports the session as running without an exit status
    assert_eq!(running, Some(None));
    // @step And an unknown session reports no session
    assert_eq!(global_store().try_wait("bug363-unknown").await?, None);
    tool.call(UnifiedExecArgs(
        serde_json::json!({"action": "close", "session_id": session}),
    ))
    .await?;
    Ok(())
}
