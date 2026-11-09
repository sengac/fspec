# MCP-005 design: structured stdio MCP command arguments

## Problem
`connect_stdio` received one `command` string and split it with `split_whitespace`, so arguments with
spaces, empty arguments and executables in directories with spaces could not be expressed. The Paseo
provider contract already sends `{ command, args?, env? }`.

## Decision
- New typed command `codelet_tools::mcp::stdio_command::McpStdioCommand { program, args }`, used by
  both front doors. `connect_stdio` takes it and passes `program` to `Command::new` and every `args`
  value to `.args(..)` literally: no shell, no splitting, no reparsing.
- Daemon wire (no backward compatibility, per the user): `SessionMcpTransport::Stdio { command, args, env }`
  — `command` is the executable itself, `args` the ordered argument vector (bincode order command, args,
  env). `HostMcpTransport::Stdio` mirrors it. Debug redacts both and prints only env keys; `redact` removes
  the executable, every argument and every env value from errors.
- ConnectMCP (LLM-facing) gains `args: Option<Vec<String>>`. With `args`, `command` is the executable
  literally (precedence is explicit: the presence of `args` selects the structured form). Without `args`,
  `command` keeps its documented whitespace-split form (`McpStdioCommand::from_command_line`), so agents
  that write `npx -y server` keep working. The field docs state both forms.
- Working directory is unchanged (the daemon/agent process cwd); env overrides are passed as before.

## Tests
`rust/rpc-server/tests/mcp005_stdio_structured_arguments.rs`: the fixture server gains `--argv`, whose
`search` tool returns its received arguments as JSON, so tests assert real argv round trips through the
daemon and the ConnectMCP tool. The MCP-003 test support's `http_fixture` moves to its own fragment so
each test binary includes only what it uses.
