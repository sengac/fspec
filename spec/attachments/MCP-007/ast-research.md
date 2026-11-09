# AST research: stdio command construction

- `fspec astgrep --pattern 'McpStdioCommand::from_command_line($$$A)' --lang rust --path rust/tools/src`: `mcp/stdio_command.rs` (definition and the call from `from_connect_args`) and `mcp/helper_tests_02.rs:40`.
- `fspec astgrep --pattern 'McpStdioCommand::from_connect_args($$$A)' --lang rust --path rust/tools/src`: `mcp/connect_tool.rs:125`.
- `$X.unwrap_or_default()` in `stdio_command.rs:22`: the empty-program default.
- The ConnectMCP input schema lives with `connect_tool.rs`. Its `args` field is `Option<Vec<String>>` and should become required for the stdio transport.
