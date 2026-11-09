use serde::{Deserialize, Serialize};

/// TOOL-027: a control sent to a running exec session's command instead
/// of a line of input. Variant order is wire order (bincode index).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ExecStdinControl {
    /// SIGINT to the command only (never the agent session or its turn).
    Interrupt,
    /// End of input: the command's stdin reaches end-of-file.
    Eof,
}
