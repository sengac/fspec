//! BUG-362 — acknowledged writes to an exec session's stdin.
//!
//! Feature: spec/features/exec-stdin-write-acknowledgement.feature
//!
//! Every write travels to the session's stdin forwarder with a oneshot
//! acknowledgement. The forwarder replies with the real io result (write and
//! flush), so a command that closed its stdin, a missing stdin pipe or a
//! failed PTY write is reported to the caller instead of being dropped.

use tokio::sync::{mpsc, oneshot};

/// Queue depth of each exec session's stdin forwarder.
pub(super) const STDIN_QUEUE_DEPTH: usize = 64;

/// One write to an exec session's stdin and where to report its result.
pub struct StdinWrite {
    pub bytes: Vec<u8>,
    pub ack: oneshot::Sender<Result<(), String>>,
}

/// The sending half of an exec session's stdin forwarder.
pub type StdinSender = mpsc::Sender<StdinWrite>;

/// The receiving half, owned by the forwarder task.
pub(super) type StdinReceiver = mpsc::Receiver<StdinWrite>;

pub(super) fn stdin_channel() -> (StdinSender, StdinReceiver) {
    mpsc::channel(STDIN_QUEUE_DEPTH)
}

/// Report a forwarder's write result back to the waiting writer. A writer
/// that stopped waiting (its future was dropped) can no longer be told.
pub(super) fn acknowledge(ack: oneshot::Sender<Result<(), String>>, result: Result<(), String>) {
    if let Err(unreported) = ack.send(result) {
        tracing::debug!(
            result = ?unreported,
            "exec stdin write finished after its writer stopped waiting"
        );
    }
}

/// Write `bytes` to exec session `exec_session`'s stdin and wait until the
/// forwarder has written and flushed them. Errors name the exec session.
pub async fn write_stdin(
    sender: &StdinSender,
    exec_session: &str,
    bytes: Vec<u8>,
) -> Result<(), String> {
    let (ack, acknowledged) = oneshot::channel();
    sender
        .send(StdinWrite { bytes, ack })
        .await
        .map_err(|_| format!("Exec session {exec_session} stdin is closed"))?;
    acknowledged
        .await
        .map_err(|_| format!("Exec session {exec_session} stdin forwarder stopped before writing"))?
        .map_err(|error| format!("Failed to write to exec session {exec_session}: {error}"))
}
