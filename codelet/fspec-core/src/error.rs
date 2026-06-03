//! Error types returned by the fspec-core dispatcher and command stubs.
//!
//! Display impls are part of the **public agent-facing contract** — the
//! exact substrings emitted here are asserted against the scenarios in
//! `spec/features/fspec-tool-rust-dispatcher.feature`. Do not rephrase
//! without updating that feature file and the dispatcher tests.

use thiserror::Error;

/// All errors produced by the fspec-core crate.
#[derive(Debug, Error)]
pub enum FspecCoreError {
    /// The command name is known and reserved for a future Rust port, but the
    /// port has not landed yet. The message MUST contain the literal
    /// substrings `"not yet ported"`, `"standalone fspec binary"`, and the
    /// porting work-unit ID so the LLM can recover and the user can navigate
    /// to the tracking card.
    #[error(
        "Command {command} is not yet ported to Rust (tracked by {work_unit}). \
         The standalone fspec binary cannot execute TypeScript fspec commands."
    )]
    NotYetPorted {
        command: &'static str,
        work_unit: &'static str,
    },

    /// The command name is not present in the canonical command list — most
    /// likely a typo or a removed command. The message MUST contain the
    /// literal substring `"Unknown fspec command"` and the offending command
    /// name, and MUST NOT contain `"not yet ported"`.
    #[error("Unknown fspec command: {command}")]
    UnknownCommand { command: String },

    /// The command name is known and ported, but the supplied `args_json`
    /// could not be parsed or validated.
    #[error("Invalid args for fspec command {command}: {reason}")]
    InvalidArgs {
        command: &'static str,
        reason: String,
    },
}
