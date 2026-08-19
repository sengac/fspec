//! Windows process-tree killer for the bash tool (BUG-156).
//!
//! Windows has no process groups. To terminate a spawned shell and every
//! process it started, we run `taskkill /PID <pid> /T` (graceful) and, when
//! forced, `taskkill /PID <pid> /T /F` — the same guard pattern as the Unix
//! `ProcessGroupKiller` (kill() + Drop). Patterns adopted from VTCode
//! (vtcode-bash-runner process_group.rs).

use std::process::Stdio;
use tracing::warn;

use crate::bash_process::taskkill_args;

/// Guard that kills the entire Windows process tree when dropped.
///
/// On drop, runs `taskkill /PID <pid> /T /F` so the shell AND all processes
/// it spawned are terminated (e.g. `npm run dev` spawning `node`).
pub struct WindowsProcessTreeKiller {
    /// PID of the spawned shell process.
    pid: Option<u32>,
}

impl WindowsProcessTreeKiller {
    /// Create a new WindowsProcessTreeKiller from a Child handle.
    pub fn new(child: &tokio::process::Child) -> Self {
        Self { pid: child.id() }
    }

    /// Explicitly kill the process tree (forceful: `taskkill /PID <pid> /T /F`).
    pub fn kill(&self) {
        if let Some(pid) = self.pid {
            let status = std::process::Command::new("taskkill")
                .args(taskkill_args(pid, true))
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            match status {
                Ok(s) if s.success() => {}
                Ok(_) => warn!("taskkill /F for pid {pid} exited unsuccessfully"),
                Err(e) => warn!("failed to run taskkill for pid {pid}: {e}"),
            }
        }
    }
}

impl Drop for WindowsProcessTreeKiller {
    fn drop(&mut self) {
        self.kill();
    }
}
