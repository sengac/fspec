#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/bash-tool-progress-session-key-streaming.feature
//!
//! RPC-398: Bash/tool output must stream incrementally into the Rust TUI.
//!
//! Root cause under test: the tool-progress callback registry
//! (`rust/tools/src/tool_progress.rs` + `session_registry.rs`) is keyed
//! by session `Uuid` with an EXACT HashMap lookup. `BashTool` emits progress
//! under the real per-session id it was built with (`BashTool::new(session_id)`
//! -> `bash_streams::emit_tool_progress(session_id, ...)`). A callback is only
//! ever invoked when it is registered under the SAME session id the tool emits
//! with.
//!
//! These behavioral tests exercise the REAL emit path: they register a
//! callback under a real UUID `S`, build `BashTool::new(S)`, invoke it via the
//! rig `Tool::call` trait, and assert the callback captured the streamed lines.
//! Existing tests only used `Uuid::nil()` + `call_with_streaming`, which
//! bypasses the registry entirely (an explicit callback is passed) and so
//! missed the registration/emit key-agreement contract.

use codelet_tools::bash::{BashArgs, BashTool};
use codelet_tools::{set_tool_progress_callback, ToolProgressCallback};
use rig::tool::Tool;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

/// Serialize tests that touch the shared bash abort flag / global process
/// spawning to reduce cross-test interference.
/// Uses an async-aware mutex so the guard can be held across await points.
static RPC398_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());

/// Collector for progress deliveries captured through the tool-progress
/// registry callback.
#[derive(Clone)]
struct ProgressCollector {
    events: Arc<Mutex<Vec<(String, bool)>>>,
}

impl ProgressCollector {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn callback(&self) -> ToolProgressCallback {
        let events = self.events.clone();
        Arc::new(move |chunk: &str, is_stderr: bool| {
            events
                .lock()
                .expect("progress collector mutex poisoned")
                .push((chunk.to_string(), is_stderr));
        })
    }

    fn all_text(&self) -> String {
        self.events
            .lock()
            .expect("progress collector mutex poisoned")
            .iter()
            .map(|(chunk, _)| chunk.as_str())
            .collect::<String>()
    }

    fn stdout_deliveries(&self) -> usize {
        self.events
            .lock()
            .expect("progress collector mutex poisoned")
            .iter()
            .filter(|(_, is_stderr)| !*is_stderr)
            .count()
    }
}

// ===========================================================================
// Scenario: Progress reaches a callback registered under the same session id
//           the tool emits with
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn progress_reaches_callback_registered_under_same_session_id() {
    let _lock = RPC398_LOCK.lock().await;

    // @step Given a tool-progress callback is registered under a real session id S
    let s = Uuid::new_v4();
    let collector = ProgressCollector::new();
    set_tool_progress_callback(s, Some(collector.callback()));

    // @step And a BashTool is built with the same session id S
    let tool = BashTool::new(s);

    // @step When the BashTool runs a command that produces output lines
    let args = serde_json::json!({
        "command": "printf 'line1\\nline2\\nline3\\n'"
    });
    let args: BashArgs = serde_json::from_value(args).expect("valid bash args");
    let result = tool.call(args).await;

    // @step Then the callback registered under S receives each output line as it is produced
    assert!(result.is_ok(), "bash command should succeed: {result:?}");
    let streamed = collector.all_text();
    assert!(
        streamed.contains("line1"),
        "callback under S must receive line1; got: {streamed:?}"
    );
    assert!(
        streamed.contains("line2"),
        "callback under S must receive line2; got: {streamed:?}"
    );
    assert!(
        streamed.contains("line3"),
        "callback under S must receive line3; got: {streamed:?}"
    );

    set_tool_progress_callback(s, None);
}

// ===========================================================================
// Scenario: Session isolation - a callback does not receive another session's
//           progress
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn callback_does_not_receive_another_sessions_progress() {
    let _lock = RPC398_LOCK.lock().await;

    // @step Given a tool-progress callback is registered under session id A
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let collector = ProgressCollector::new();
    set_tool_progress_callback(a, Some(collector.callback()));
    // Ensure B has no callback of its own.
    set_tool_progress_callback(b, None);

    // @step And a BashTool is built with a different session id B
    let tool = BashTool::new(b);

    // @step When the BashTool built with session id B runs a command that produces output
    let args = serde_json::json!({
        "command": "printf 'isolated1\\nisolated2\\n'"
    });
    let args: BashArgs = serde_json::from_value(args).expect("valid bash args");
    let result = tool.call(args).await;
    assert!(result.is_ok(), "bash command should succeed: {result:?}");

    // @step Then the callback registered under session id A receives no progress
    let streamed = collector.all_text();
    assert!(
        streamed.is_empty(),
        "callback under A must NOT receive progress emitted by BashTool(B); got: {streamed:?}"
    );

    set_tool_progress_callback(a, None);
}

// ===========================================================================
// Scenario: Incremental progress is delivered while the command is still
//           running
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn incremental_progress_delivered_while_command_running() {
    let _lock = RPC398_LOCK.lock().await;

    // @step Given a tool-progress callback is registered under a real session id S
    let s = Uuid::new_v4();
    let collector = ProgressCollector::new();
    set_tool_progress_callback(s, Some(collector.callback()));

    // @step And a BashTool is built with the same session id S
    let tool = BashTool::new(s);

    // @step When the BashTool runs a command that prints three lines with delays between them
    let args = serde_json::json!({
        "command": "echo a; sleep 0.1; echo b; sleep 0.1; echo c"
    });
    let args: BashArgs = serde_json::from_value(args).expect("valid bash args");
    let result = tool.call(args).await;
    assert!(result.is_ok(), "bash command should succeed: {result:?}");

    // @step Then the callback receives three incremental progress deliveries before the final tool result
    // Prove per-line streaming robustly: at least three incremental deliveries
    // arrived (tolerating benign line-buffer coalescing under load), AND all
    // three lines "a", "b", "c" were streamed as their own output.
    let deliveries = collector.stdout_deliveries();
    let streamed = collector.all_text();
    assert!(
        deliveries >= 3,
        "callback must receive at least three incremental stdout deliveries; \
         got {deliveries} deliveries with text: {streamed:?}"
    );
    assert!(
        streamed.contains("a") && streamed.contains("b") && streamed.contains("c"),
        "streamed stdout must contain all three lines 'a', 'b', 'c'; got: {streamed:?}"
    );

    set_tool_progress_callback(s, None);
}

// ===========================================================================
// Scenario: Clearing the callback removes the registration for that session id
// ===========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn clearing_callback_removes_registration_for_session_id() {
    let _lock = RPC398_LOCK.lock().await;

    // @step Given a tool-progress callback is registered under a real session id S
    let s = Uuid::new_v4();
    let collector = ProgressCollector::new();
    set_tool_progress_callback(s, Some(collector.callback()));

    // @step When the stream loop clears the callback for session id S
    set_tool_progress_callback(s, None);

    // @step And progress is emitted under session id S
    codelet_tools::emit_tool_progress(s, "after clear\n", false);

    // @step Then no callback is invoked for session id S
    let streamed = collector.all_text();
    assert!(
        streamed.is_empty(),
        "cleared callback under S must not be invoked; got: {streamed:?}"
    );
}
