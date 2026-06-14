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

    /// Filesystem I/O failure while executing a ported command.
    #[error("I/O error executing fspec command {command}: {source}")]
    Io {
        command: &'static str,
        #[source]
        source: std::io::Error,
    },

    /// JSON parse failure for one of the canonical fspec state files. The
    /// message MUST contain `"Failed to parse <file>"` for parity with the
    /// TypeScript implementation (`src/utils/ensure-files.ts:49-52`).
    #[error(
        "Failed to parse {file}: {reason}. The file may be corrupted or contain invalid JSON."
    )]
    ParseJson { file: String, reason: String },

    /// A required directory was not found. Used by `list-features` (RPC-245)
    /// and any future command that needs to escalate a missing-directory
    /// condition with a dedicated exit code (2 instead of the generic 1).
    /// The message MUST contain the literal substring
    /// `"Directory not found: <path>"` for parity with the TypeScript
    /// implementation (`src/commands/list-features.ts:33-38`).
    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: String },

    /// A precondition for a project-management command was not met and the
    /// command surfaces a fully-formed, agent-facing message VERBATIM (no
    /// wrapping prefix). Used by `check_foundation_exists` (Batch 11 create-*
    /// commands) to emit the foundation-missing `userMessage` +
    /// `<system-reminder>` exactly as built by the TypeScript
    /// `buildFoundationMissingError` (`src/utils/foundation-check.ts:48-89`).
    /// The message MUST contain the substrings `"Project foundation not found"`
    /// and `"<system-reminder>"`.
    #[error("{0}")]
    FoundationMissing(String),

    /// A runtime failure that surfaces a fully-formed, agent-facing message
    /// VERBATIM (no wrapping prefix). Used to reproduce the uncaught-TypeError
    /// crash messages the TypeScript `validate-work-units` `.action` catch
    /// block prints (e.g. `Cannot convert undefined or null to object`,
    /// `Cannot read properties of undefined (reading 'children')`). The CLI
    /// bridge renders these as `✗ Failed to validate work units: {self}` to
    /// match the TS reference byte-for-byte.
    #[error("{0}")]
    Message(String),
}
