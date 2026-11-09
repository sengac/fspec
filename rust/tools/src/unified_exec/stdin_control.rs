//! TOOL-027: send an interrupt or end of input to a live exec session.
//!
//! Feature: spec/features/exec-stdin-control.feature
//!
//! PTY sessions get the terminal's interrupt (0x03) and end-of-file (0x04)
//! characters, which the slave's line discipline turns into SIGINT for the
//! foreground process group and an end-of-file read. Pipe sessions have no
//! line discipline: interrupt signals the command's process group (the
//! pipe spawn uses `process_group(0)`, so the shell's pid is the pgid) and
//! end of input closes the command's stdin.

use tokio::sync::mpsc;

use super::process_store::ProcessStore;

/// The terminal interrupt character (Ctrl+C).
const PTY_INTERRUPT: u8 = 0x03;
/// The terminal end-of-file character (Ctrl+D).
const PTY_END_OF_FILE: u8 = 0x04;

/// A control sent to an exec session's command instead of input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdinControl {
    /// SIGINT to the command.
    Interrupt,
    /// End of input on the command's stdin.
    Eof,
}

/// What `control_stdin` does once the store lock is released.
enum Delivery {
    /// Write one control byte to a PTY session's stdin.
    Byte(super::stdin_write::StdinSender, u8),
    /// SIGINT to a pipe session's process group.
    Signal(u32),
    /// A pipe session's stdin was closed under the lock.
    Closed,
}

impl ProcessStore {
    /// Deliver `control` to exec session `session_id`. Errors name the
    /// exec session.
    pub async fn control_stdin(
        &self,
        session_id: &str,
        control: StdinControl,
    ) -> Result<(), String> {
        let delivery = {
            let mut entries = self.entries.lock().await;
            let entry = entries
                .get_mut(session_id)
                .ok_or_else(|| format!("Unknown or exited exec session: {session_id}"))?;
            match (entry.tty, control) {
                (true, StdinControl::Interrupt) => {
                    Delivery::Byte(entry.stdin_tx.clone(), PTY_INTERRUPT)
                }
                (true, StdinControl::Eof) => {
                    Delivery::Byte(entry.stdin_tx.clone(), PTY_END_OF_FILE)
                }
                (false, StdinControl::Interrupt) => Delivery::Signal(
                    entry
                        .child
                        .id()
                        .ok_or_else(|| format!("Exec session {session_id} has already exited"))?,
                ),
                (false, StdinControl::Eof) => {
                    // The store holds the only long-lived sender. Swapping in
                    // a sender whose receiver is already gone drops it, so the
                    // stdin forwarder's recv loop ends and drops the child's
                    // stdin (end of file). Later writes fail on the closed one.
                    let (closed, receiver) = mpsc::channel(1);
                    drop(receiver);
                    drop(std::mem::replace(&mut entry.stdin_tx, closed));
                    Delivery::Closed
                }
            }
        };
        match delivery {
            Delivery::Byte(sender, byte) => {
                super::stdin_write::write_stdin(&sender, session_id, vec![byte]).await
            }
            Delivery::Signal(pgid) => interrupt_process_group(session_id, pgid).await,
            Delivery::Closed => Ok(()),
        }
    }
}

/// SIGINT to the process group `pgid` through `kill(1)` (no unsafe code).
#[cfg(unix)]
async fn interrupt_process_group(session_id: &str, pgid: u32) -> Result<(), String> {
    let status = tokio::process::Command::new("kill")
        .args(["-INT", "--", &format!("-{pgid}")])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
        .map_err(|error| format!("Failed to run kill for exec session {session_id}: {error}"))?;
    if status.status.success() {
        return Ok(());
    }
    Err(format!(
        "kill -INT for exec session {session_id} failed ({}): {}",
        status.status,
        String::from_utf8_lossy(&status.stderr).trim()
    ))
}

/// Windows has no SIGINT for a piped child without a console.
#[cfg(not(unix))]
async fn interrupt_process_group(session_id: &str, _pgid: u32) -> Result<(), String> {
    Err(format!(
        "Interrupting pipe exec session {session_id} is not supported on Windows"
    ))
}
