//! RIG-016: how input submitted to a busy session reaches its agent.

use serde::{Deserialize, Serialize};

/// The delivery mode of input submitted while a session's turn may be running.
///
/// An idle session treats every mode as an ordinary new turn.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, schemars::JsonSchema,
)]
pub enum InputMode {
    /// Join the running turn at its next safe boundary (after the current
    /// model step and every pending tool result, before the next request).
    #[default]
    Steer,
    /// Start as a new turn when the running turn completes, in order.
    Queue,
    /// Cancel the running turn and park the input until the user resumes.
    Interrupt,
}
