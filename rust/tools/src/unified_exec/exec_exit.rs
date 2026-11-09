//! BUG-368: how an exec session's command really ended.

use std::sync::{Arc, Mutex};

use super::types::UnifiedExecResult;

/// The end state of an exec session's command: its exit code, the signal
/// that terminated it, or neither when the status could not be recovered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecExit {
    pub code: Option<i32>,
    pub signal: Option<String>,
}

/// Where a PTY session's wait task records the PTY child's status. The
/// store's `Child` for a PTY session is only a liveness anchor, so its own
/// status says nothing about the command.
pub type PtyExitSlot = Arc<Mutex<Option<ExecExit>>>;

impl ExecExit {
    /// The status could not be recovered.
    pub fn unavailable() -> Self {
        Self {
            code: None,
            signal: None,
        }
    }

    /// From a pipe session's (or the anchor's) `std` exit status.
    pub fn from_std(status: std::process::ExitStatus) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(signal) = status.signal() {
                return Self {
                    code: None,
                    signal: Some(format!("signal {signal}")),
                };
            }
        }
        Self {
            code: status.code(),
            signal: None,
        }
    }

    /// From the PTY child's status. portable-pty keeps the signal name
    /// private and renders it only through `Display` ("Terminated by X").
    pub fn from_pty(status: &portable_pty::ExitStatus) -> Self {
        let shown = status.to_string();
        if let Some(signal) = shown.strip_prefix("Terminated by ") {
            return Self {
                code: None,
                signal: Some(signal.to_string()),
            };
        }
        match i32::try_from(status.exit_code()) {
            Ok(code) => Self {
                code: Some(code),
                signal: None,
            },
            Err(_) => Self::unavailable(),
        }
    }

    /// The real end state of a session whose (anchor) `Child` exited:
    /// the PTY child's recorded status for a PTY session, else `status`.
    pub(crate) fn resolve(status: std::process::ExitStatus, pty: Option<&PtyExitSlot>) -> Self {
        let Some(slot) = pty else {
            return Self::from_std(status);
        };
        match slot.lock() {
            Ok(recorded) => recorded.clone().unwrap_or_else(Self::unavailable),
            Err(_) => {
                tracing::error!("PTY exit slot poisoned; the exit status is unavailable");
                Self::unavailable()
            }
        }
    }
}

impl UnifiedExecResult {
    /// A result for a command that ended as `exit` (no output yet).
    pub fn exited(exit: &ExecExit) -> Self {
        Self {
            exit_code: exit.code,
            signal: exit.signal.clone(),
            exit_status_unavailable: (exit.code.is_none() && exit.signal.is_none()).then_some(true),
            session_id: None,
            output: None,
            wall_time_seconds: None,
            sessions: None,
            error: None,
            quiet_seconds: None,
            raw_output: None,
        }
    }
}
