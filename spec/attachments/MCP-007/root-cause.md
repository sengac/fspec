# Root cause: ConnectMCP guesses a stdio command line

## Where
`rust/tools/src/mcp/stdio_command.rs` (added by MCP-005, commit `563c8eed`):

- `McpStdioCommand::from_connect_args(command, None)` falls back to `from_command_line(command)`, which splits the string on whitespace and treats the first token as the program and the rest as arguments.
- `from_command_line` uses `parts.next().unwrap_or_default()`, so an empty or all-whitespace command silently becomes an empty program instead of an error.
- Callers: `rust/tools/src/mcp/connect_tool.rs:125` (the ConnectMCP tool) and `rust/tools/src/mcp/helper_tests_02.rs:40`.

## Why it is wrong
- **A guess, not the given data.** Whitespace splitting cannot represent a program path or argument that contains spaces, which is exactly the bug MCP-005 fixed on the daemon path. Keeping it as a path the model can take reintroduces the ambiguity.
- **Two front doors with different rules.** The daemon (`SessionMcpTransport::Stdio`) takes `command` as the executable only. ConnectMCP's single-string form keeps the old meaning.
- **A silent default.** An empty program becomes `""`, which surfaces later as an opaque spawn failure.
- **Against the user's rules.** No backward compatibility is needed and no fallbacks are allowed.

## Fix
- ConnectMCP's stdio form takes `command` (the executable, required and non-empty) and `args` (a list, required; it may be empty).
- Remove `from_command_line`.
- An empty or whitespace-only `command`, or a missing `args` for the stdio transport, is a schema or validation error that names the field.
- Update the ConnectMCP tool schema and description, and any tests and fixtures that pass a single command line (for example `npx -y @modelcontextprotocol/server-everything`).
