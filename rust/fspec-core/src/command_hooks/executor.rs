//! Hook execution engine (HOOK-003).
//!
//! Hooks run sequentially as child processes in the project root, inheriting
//! the environment, with the JSON [`HookContext`] on stdin. Commands run under
//! `poll_sync_future`, so this is blocking `std::process`: two reader threads
//! drain stdout and stderr and report end-of-file on a channel, and the
//! hook's timeout bounds the wait for both. A hook that outlives its timeout
//! is killed and reaped.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::config::{is_script_path, CommandHook, HookConfig};
use crate::io::time::iso8601_now;

/// The JSON context a hook reads on stdin.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_unit_id: Option<String>,
    pub event: String,
    pub timestamp: String,
    /// Staged paths, for a virtual hook with `gitContext` (HOOK-019).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staged_files: Option<Vec<String>>,
    /// Unstaged paths, for a virtual hook with `gitContext` (HOOK-019).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unstaged_files: Option<Vec<String>>,
    /// The status a status hook's move leaves (HOOK-020).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_status: Option<String>,
    /// The status a status hook's move enters (HOOK-020).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_status: Option<String>,
}

impl HookContext {
    /// A context for `event`, stamped now.
    pub fn new(work_unit_id: Option<&str>, event: &str) -> Self {
        Self {
            work_unit_id: work_unit_id.map(str::to_string),
            event: event.to_string(),
            timestamp: iso8601_now(),
            staged_files: None,
            unstaged_files: None,
            from_status: None,
            to_status: None,
        }
    }

    /// The context of a status hook (global or virtual) for work unit `id`
    /// moving from `from` to `to` (HOOK-020).
    pub fn status(id: &str, event: &str, from: &str, to: &str) -> Self {
        Self {
            from_status: Some(from.to_string()),
            to_status: Some(to.to_string()),
            ..Self::new(Some(id), event)
        }
    }
}

/// What one hook run produced.
#[derive(Debug, Clone, PartialEq)]
pub struct HookResult {
    pub hook_name: String,
    pub blocking: bool,
    /// `None` when the hook timed out, was killed by a signal or could not
    /// be started.
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub timeout_secs: u64,
}

impl HookResult {
    /// Exit code 0 within the timeout.
    pub fn success(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }
}

/// Run one hook to completion (or its timeout).
pub fn execute_hook(
    project_root: &Path,
    config: &HookConfig,
    hook: &CommandHook,
    context: &HookContext,
) -> HookResult {
    let mut result = HookResult {
        hook_name: hook.name.clone(),
        blocking: hook.blocking,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        timed_out: false,
        timeout_secs: hook.timeout_secs,
    };
    let mut command = if is_script_path(&hook.command, project_root) {
        Command::new(project_root.join(&hook.command))
    } else {
        let (shell, flag) = parse_shell(config.shell.as_deref());
        let mut command = Command::new(shell);
        command.arg(flag).arg(&hook.command);
        command
    };
    let spawned = command
        .current_dir(project_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => {
            result.stderr = format!("Failed to run hook '{}': {e}", hook.name);
            return result;
        }
    };

    let (done_tx, done_rx) = mpsc::channel();
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        &done_tx,
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn Read + Send>),
        &done_tx,
    );
    drop(done_tx);
    write_context(&mut child, context);

    let deadline = Instant::now() + Duration::from_secs(hook.timeout_secs);
    let mut open_streams = 2;
    while open_streams > 0 {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match done_rx.recv_timeout(remaining) {
            Ok(()) => open_streams -= 1,
            Err(RecvTimeoutError::Timeout) => {
                result.timed_out = true;
                break;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    if result.timed_out {
        // A hook that cannot be killed has already exited; wait reaps it.
        let _ = child.kill();
    }
    match child.wait() {
        Ok(status) if !result.timed_out => result.exit_code = status.code(),
        Ok(_) => {}
        Err(e) => result.stderr = format!("Failed to wait for hook '{}': {e}", hook.name),
    }
    result.stdout = text(&stdout);
    let stderr = text(&stderr);
    if result.stderr.is_empty() {
        result.stderr = stderr;
    } else if !stderr.is_empty() {
        result.stderr = format!("{stderr}\n{}", result.stderr);
    }
    result
}

/// Run `hooks` sequentially in order.
pub fn execute_hooks(
    project_root: &Path,
    config: &HookConfig,
    hooks: &[CommandHook],
    context: &HookContext,
) -> Vec<HookResult> {
    hooks
        .iter()
        .map(|hook| execute_hook(project_root, config, hook, context))
        .collect()
}

/// Write the context JSON (one line) to the hook's stdin and close it. A
/// hook that exits without reading stdin closes the pipe first; that is not
/// a failure of the hook, so the write error is dropped.
fn write_context(child: &mut Child, context: &HookContext) {
    if let Some(mut stdin) = child.stdin.take() {
        if let Ok(json) = serde_json::to_string(context) {
            let _ = stdin.write_all(format!("{json}\n").as_bytes());
        }
    }
}

type Captured = Arc<Mutex<Vec<u8>>>;

/// Drain `stream` on its own thread into a shared buffer, signalling `done`
/// at end-of-file (or a read error).
fn drain(stream: Option<Box<dyn Read + Send>>, done: &mpsc::Sender<()>) -> Captured {
    let captured: Captured = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&captured);
    let done = done.clone();
    std::thread::spawn(move || {
        if let Some(mut stream) = stream {
            let mut chunk = [0u8; 8192];
            while let Ok(n) = stream.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                match sink.lock() {
                    Ok(mut buf) => buf.extend_from_slice(&chunk[..n]),
                    Err(poisoned) => poisoned.into_inner().extend_from_slice(&chunk[..n]),
                }
            }
        }
        let _ = done.send(());
    });
    captured
}

fn text(captured: &Captured) -> String {
    let bytes = match captured.lock() {
        Ok(buf) => buf.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    String::from_utf8_lossy(&bytes).into_owned()
}

/// `global.shell` as program and flag: `"bash -c"` gives `("bash", "-c")`,
/// `"/bin/bash"` gives `("/bin/bash", "-c")`, none gives `("sh", "-c")`
/// (the convention of codelet-core's agent lifecycle hooks).
fn parse_shell(shell: Option<&str>) -> (String, String) {
    match shell.map(str::trim) {
        Some(s) if !s.is_empty() => match s.split_once(' ') {
            Some((program, flag)) => (program.to_string(), flag.trim().to_string()),
            None => (s.to_string(), "-c".to_string()),
        },
        _ => ("sh".to_string(), "-c".to_string()),
    }
}
