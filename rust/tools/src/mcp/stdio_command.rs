//! MCP-005: the typed command that launches a stdio MCP server.
//!
//! Feature: spec/features/mcp-stdio-structured-arguments.feature
//!
//! The program and every argument are passed to the process literally:
//! nothing is split, quoted, expanded or interpreted by a shell.

/// An executable and its ordered, literal argument vector.
#[derive(Clone, PartialEq, Eq)]
pub struct McpStdioCommand {
    /// The executable (a name looked up on PATH, or a path, spaces allowed).
    pub program: String,
    /// The arguments, each passed as one argv entry.
    pub args: Vec<String>,
}

impl McpStdioCommand {
    /// MCP-007: ConnectMCP's stdio command. `command` is the executable itself
    /// (required, non-empty) and `args` its literal argument list (required,
    /// may be empty). Nothing is split on whitespace.
    pub fn from_connect_args(
        command: Option<&str>,
        args: Option<&[String]>,
    ) -> Result<Self, crate::ToolError> {
        let program =
            command.ok_or_else(|| connect_validation("command is required for stdio transport"))?;
        if program.trim().is_empty() {
            return Err(connect_validation(
                "command must be a non-empty executable for stdio transport",
            ));
        }
        let args = args.ok_or_else(|| {
            connect_validation("args is required for stdio transport (use [] for no arguments)")
        })?;
        Ok(Self {
            program: program.to_string(),
            args: args.to_vec(),
        })
    }
}

fn connect_validation(message: &str) -> crate::ToolError {
    crate::ToolError::Validation {
        tool: "ConnectMCP",
        message: message.to_string(),
    }
}

/// Debug never prints the program or its arguments (they may carry secrets).
impl std::fmt::Debug for McpStdioCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpStdioCommand")
            .field("program", &"<redacted>")
            .field("arg_count", &self.args.len())
            .finish()
    }
}
