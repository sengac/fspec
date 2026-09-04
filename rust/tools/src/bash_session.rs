//! TOOL-022 P4 (G1/G2/G3/G4) — BashTool execution via the unified_exec
//! session machinery.
//!
//! Before P4, the Bash tool spawned its own short-lived child
//! (`sh -c command` with inherited stdin — G1), had no pager env
//! suppression (G2), and blocked to exit with no session handle. Only
//! the Codex provider saw the unified exec session surface, so the
//! TOOL-022 exec-stdin overlay (the TUI ⌨ composer prompt + the LLM
//! quiet-seconds steering) only ever appeared on Codex sessions.
//!
//! P4 makes the Bash tool a THIN wrapper over `UnifiedExecTool`:
//!
//! - one `run` (bounded initial yield) + a bounded `poll` loop until
//!   the child exits, honouring the per-session bash abort flag
//!   (`request_bash_abort` / ESC) — on abort the whole process tree
//!   dies via the session's `ChildHandle` (process group / taskkill);
//! - the merged stdout+stderr bytes are split back on the
//!   `STDERR_MARKER` prefix (`bash_output::split_merged_output`) so the
//!   existing `BashOutput` formatting (stderr marking, truncation) is
//!   UNCHANGED for all providers;
//! - the BUG-142 binary-output guard still runs on the raw stdout
//!   bytes;
//! - while the command is still running the TOOL-022 P1 quiet
//!   detector has already started (it is spawned with the reaper in
//!   `handle_run`), so the TUI exec-stdin overlay surfaces for ANY
//!   provider's Bash session; the result the LLM sees carries
//!   `quiet_seconds` + the fixed steering line, and a delegation footer
//!   teaches it the session id + how to send stdin.
//!
//! The `Tool` impl name stays `Bash` (no schema change) — providers
//! keep their existing tool names.

use serde_json::json;

use rig::tool::Tool;

use crate::bash_abort::is_bash_abort_requested;
use crate::bash_binary_guard::{detect_bash_binary_output, format_binary_guard_message};
use crate::bash_output::{split_merged_output_bytes, BashOutput, STDERR_MARKER};
use crate::bash_process::ABORT_MESSAGE;
use crate::bash_streams::StreamCallback;
use crate::error::ToolError;
use crate::tool_progress::emit_tool_progress;
use crate::unified_exec::{
    clamp_bash_delegation_yield_time, global_store, poll_session_interruptible, UnifiedExecArgs,
    UnifiedExecTool,
};
use uuid::Uuid;

/// Bounded initial yield for the Bash delegation `run` (ms) — the
/// `clamp_yield_time` minimum, keeping the first-output latency low
/// while still letting the P1/P2 detector + quiet timestamp start.
const BASH_RUN_YIELD_MS: u64 = 250;

/// Bounded per-iteration poll yield for the Bash delegation loop (ms)
/// — also the cadence of the abort-flag check. Below the LLM-facing
/// poll clamp minimum on purpose: delegation polls are driven by our
/// abort loop, not by the LLM (see
/// `clamp_bash_delegation_yield_time`).
const BASH_POLL_YIELD_MS: u64 = 150;

/// Result of a delegated Bash execution.
pub(crate) struct BashSessionResult {
    /// Raw stdout bytes (binary-guard input).
    pub stdout_bytes: Vec<u8>,
    /// Reconstructed stderr text (marker-stripped).
    pub stderr: String,
    /// Exit code (`None` when the process could not be waited on).
    pub exit_code: Option<i32>,
}

/// Build the one-shot Bash result from a completed session
/// (stdout/stderr + exit code), applying the BUG-142 binary-output
/// guard to the raw stdout bytes.
pub(crate) fn finalize_bash_result(result: BashSessionResult) -> Result<String, ToolError> {
    if let Some(kind) = detect_bash_binary_output(&result.stdout_bytes) {
        return Err(ToolError::Execution {
            tool: "bash",
            message: format_binary_guard_message(kind),
        });
    }
    let stdout = String::from_utf8_lossy(&result.stdout_bytes).into_owned();
    let exit_code = result.exit_code.unwrap_or(-1);
    let bash_output = BashOutput {
        stdout,
        stderr: result.stderr,
        exit_code: result.exit_code,
        success: exit_code == 0,
    };
    if exit_code == 0 {
        Ok(bash_output.format_success())
    } else {
        Err(ToolError::Execution {
            tool: "bash",
            message: bash_output.format_error(),
        })
    }
}

/// One delegation pass: `run` then a single bounded `poll` window.
///
/// Returns `Some(result)` when the command finished inside the window;
/// `None` when it is still running (the caller gets the partial merged
/// output + the still-running session id to continue or surface).
pub(crate) async fn run_bash_one_pass(
    session_id: Uuid,
    command: &str,
    cwd: Option<&str>,
    poll_yield_ms: u64,
) -> Result<OnePassResult, ToolError> {
    let tool = UnifiedExecTool::new(session_id)
        .with_abort_check(move || is_bash_abort_requested(session_id));
    let run_args = json!({
        "action": "run",
        "command": command,
        "workdir": cwd,
        "yield_time_ms": BASH_RUN_YIELD_MS,
        "skip_blocklist": true,
    });
    let run = tool.call(UnifiedExecArgs(run_args)).await?;
    let Some(exec_session) = run.session_id else {
        // Fast command — exited within the bounded initial yield.
        // `raw_output` carries the marker-tagged bytes; the BYTE-LEVEL
        // split keeps the stdout half intact for the BUG-142 guard.
        // The raw bytes are preserved on the result so the caller can
        // still stream them to the UI (pre-P4 contract: every line
        // reaches the callback).
        let raw = run.raw_output.unwrap_or_default();
        let (stdout_bytes, stderr_bytes) = split_merged_output_bytes(&raw);
        return Ok(OnePassResult {
            finished: true,
            exec_session: None,
            merged_so_far: raw,
            result: BashSessionResult {
                stdout_bytes,
                stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                exit_code: run.exit_code,
            },
        });
    };

    // The run's initial yield window already drained bytes into the
    // result — seed the accumulated merged output with them, THEN take
    // the first bounded poll window.
    let mut merged_so_far = run.raw_output.clone().unwrap_or_default();
    let (chunk, code) = drain_bash_output(session_id, &exec_session, poll_yield_ms).await?;
    merged_so_far.extend_from_slice(&chunk);
    if let Some(code) = code {
        let (stdout_bytes, stderr_bytes) = split_merged_output_bytes(&merged_so_far);
        return Ok(OnePassResult {
            finished: true,
            exec_session: None,
            merged_so_far: Vec::new(),
            result: BashSessionResult {
                stdout_bytes,
                stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                exit_code: Some(code),
            },
        });
    }
    Ok(OnePassResult {
        finished: false,
        exec_session: Some(exec_session),
        merged_so_far,
        result: BashSessionResult {
            stdout_bytes: Vec::new(),
            stderr: String::new(),
            exit_code: None,
        },
    })
}

/// Outcome of a single delegation pass.
pub(crate) struct OnePassResult {
    /// True when the command finished within the pass.
    pub finished: bool,
    /// The exec session id while still running.
    pub exec_session: Option<String>,
    /// Raw marker-tagged merged bytes captured so far — kept verbatim
    /// (no lossy decode) so the final byte-level split + the binary
    /// guard see every stdout byte exactly once.
    pub merged_so_far: Vec<u8>,
    /// The finished result (only when `finished`).
    pub result: BashSessionResult,
}

/// Poll a still-running delegated session for new marker-tagged output.
///
/// Bypasses the LLM-facing poll clamp on purpose — this is the
/// delegation loop's own cadence (`clamp_bash_delegation_yield_time`),
/// and the drain observes the per-session abort flag every ~50ms
/// (mid-window), so an ESC abort terminates within ~`min(poll window,
/// 50ms)` + flag skew (the pre-P4 contract terminates within ~200ms).
///
/// Returns `(raw marker-tagged merged chunk, exit_code_when_finished)`
/// — RAW BYTES so the stdout half stays binary-safe through the final
/// split + the BUG-142 guard.
pub(crate) async fn drain_bash_output(
    session_id: Uuid,
    exec_session: &str,
    yield_ms: u64,
) -> Result<(Vec<u8>, Option<i32>), ToolError> {
    let yield_ms = clamp_bash_delegation_yield_time(yield_ms);
    let interrupt: std::sync::Arc<dyn Fn() -> bool + Send + Sync> =
        std::sync::Arc::new(move || is_bash_abort_requested(session_id));
    let result = poll_session_interruptible(exec_session, yield_ms, &interrupt).await?;
    Ok((result.raw_output.unwrap_or_default(), result.exit_code))
}

/// Kill the exec session's process tree + remove it from the store.
/// Used by the abort path (ESC).
async fn abort_exec_session(exec_session: &str) {
    let store = global_store();
    if let Some(kill) = store.kill_handle(exec_session).await {
        kill.kill();
    }
    // Close (kills + removes the entry). A missing entry (the reaper
    // or a prior poll drained it first) is not an error on the abort
    // path.
    let _ = store.close_session(exec_session).await;
}

/// Abort the exec session from a streaming context (ESC mid-stream).
pub(crate) async fn abort_bash_session(exec_session: &str) {
    abort_exec_session(exec_session).await;
}

/// UI stream destination for a delegated Bash command (mirrors the
/// pre-P4 `StdoutStreamMode` split): the `Tool` impl streams stdout
/// AND stderr (red) via tool progress; the streaming callback path
/// streams stdout lines only to the callback (stderr stays in the
/// final result, unchanged).
pub(crate) enum BashUiStream {
    /// Stream via `emit_tool_progress` (the `Tool::call` path).
    ToolProgress,
    /// Stream stdout lines via an explicit callback
    /// (`call_with_streaming`).
    Callback(StreamCallback),
}

/// Forward one merged-output chunk to the UI, line-by-line, splitting
/// stderr lines (the `⚠stderr⚠` marker) for red styling. The callback
/// destination never receives stderr (pre-P4 streaming contract).
///
/// Works on the RAW bytes; lines are lossy-decoded for display only.
fn stream_chunk(ui: &BashUiStream, session_id: Uuid, chunk: &[u8]) {
    let marker = STDERR_MARKER.as_bytes();
    for line in chunk.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        match line.strip_prefix(marker) {
            Some(rest) => match ui {
                BashUiStream::ToolProgress => {
                    let text = format!("{}\n", String::from_utf8_lossy(rest));
                    emit_tool_progress(session_id, &text, true)
                }
                BashUiStream::Callback(_) => {}
            },
            None => match ui {
                BashUiStream::ToolProgress => {
                    let text = format!("{}\n", String::from_utf8_lossy(line));
                    emit_tool_progress(session_id, &text, false)
                }
                BashUiStream::Callback(cb) => {
                    let text = format!("{}\n", String::from_utf8_lossy(line));
                    cb(&text)
                }
            },
        }
    }
}

/// Run `command` via the unified exec session machinery, streaming
/// drained output to `ui` while it runs, until the child exits or the
/// user aborts (ESC → per-session abort flag → process-tree kill).
///
/// This is the single execution path for `BashTool::call` and
/// `BashTool::call_with_streaming` (TOOL-022 P4) — the pre-P4
/// own-child spawn (inherited stdin, no pager env, no session handle)
/// is gone. While the command is still running the P2 exec-stdin
/// detector has been live since the initial `run` (it is spawned with
/// the reaper), so the TUI ⌨ overlay surfaces for any provider.
pub(crate) async fn run_bash_session(
    session_id: Uuid,
    command: &str,
    cwd: Option<&str>,
    ui: BashUiStream,
) -> Result<BashSessionResult, ToolError> {
    let first = run_bash_one_pass(session_id, command, cwd, BASH_POLL_YIELD_MS).await?;
    if first.finished {
        // Stream the fast path's captured output to the UI as well —
        // pre-P4 every line reached the callback/tool-progress while
        // the child ran, so the delegated fast path must do the same.
        stream_chunk(&ui, session_id, &first.merged_so_far);
        return Ok(first.result);
    }
    let exec_session = first.exec_session.ok_or_else(|| ToolError::Execution {
        tool: "bash",
        message: "delegated exec session lost".to_string(),
    })?;
    let mut merged = first.merged_so_far;
    stream_chunk(&ui, session_id, &merged);

    loop {
        if is_bash_abort_requested(session_id) {
            abort_bash_session(&exec_session).await;
            return Err(ToolError::Execution {
                tool: "bash",
                message: ABORT_MESSAGE.to_string(),
            });
        }
        let (chunk, code) =
            drain_bash_output(session_id, &exec_session, BASH_POLL_YIELD_MS).await?;
        if !chunk.is_empty() {
            stream_chunk(&ui, session_id, &chunk);
            merged.extend_from_slice(&chunk);
        }
        if let Some(code) = code {
            let (stdout_bytes, stderr_bytes) = split_merged_output_bytes(&merged);
            return Ok(BashSessionResult {
                stdout_bytes,
                stderr: String::from_utf8_lossy(&stderr_bytes).into_owned(),
                exit_code: Some(code),
            });
        }
    }
}
