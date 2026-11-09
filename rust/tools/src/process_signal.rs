//! BUG-365: safe process signalling (SIGKILL to a process or a process
//! group) through `nix`, with every result reported.

use nix::errno::Errno;
use nix::sys::signal::{kill, killpg, Signal};
use nix::unistd::Pid;

/// What a kill did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillOutcome {
    /// The signal was delivered.
    Signalled,
    /// The target had already exited; there was nothing to kill.
    AlreadyGone,
}

/// A kill that failed for a reason other than the target being gone.
#[derive(Debug, thiserror::Error)]
pub enum KillError {
    #[error("pid {0} is out of range for a signal target")]
    PidOutOfRange(u32),
    #[error("cannot kill {target} {pid}: {errno}")]
    Signal {
        target: &'static str,
        pid: u32,
        errno: Errno,
    },
}

fn target_pid(pid: u32) -> Result<Pid, KillError> {
    i32::try_from(pid)
        .ok()
        .filter(|raw| *raw > 0)
        .map(Pid::from_raw)
        .ok_or(KillError::PidOutOfRange(pid))
}

fn outcome(
    result: nix::Result<()>,
    target: &'static str,
    pid: u32,
) -> Result<KillOutcome, KillError> {
    match result {
        Ok(()) => Ok(KillOutcome::Signalled),
        Err(Errno::ESRCH) => Ok(KillOutcome::AlreadyGone),
        Err(errno) => Err(KillError::Signal { target, pid, errno }),
    }
}

/// SIGKILL one process.
pub fn kill_process(pid: u32) -> Result<KillOutcome, KillError> {
    outcome(kill(target_pid(pid)?, Signal::SIGKILL), "process", pid)
}

/// SIGKILL a whole process group (the leader's pid is the group id).
pub fn kill_process_group(pgid: u32) -> Result<KillOutcome, KillError> {
    outcome(
        killpg(target_pid(pgid)?, Signal::SIGKILL),
        "process group",
        pgid,
    )
}

/// Kill and log a failure — for kill paths (drop, kill handles) that
/// have no caller to return an error to.
pub fn kill_logged(result: Result<KillOutcome, KillError>) {
    if let Err(error) = result {
        tracing::error!(%error, "process kill failed");
    }
}
