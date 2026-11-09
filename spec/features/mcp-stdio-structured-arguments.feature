@BUG-582
@done
@tool-discovery
@MCP-005
@MCP-007
@rpc
@session
@external-bridge
Feature: Structured stdio MCP command arguments
  """
  A stdio MCP server is launched from a typed McpStdioCommand { program, args }
  (rust/tools/src/mcp/stdio_command.rs): connect_stdio passes the program to Command::new and every
  argument literally, with no whitespace splitting and no shell. Both front doors build it:
  - the daemon wire type SessionMcpTransport::Stdio { command, args, env } (bincode order command, args,
  env), where command is the executable itself;
  - the ConnectMCP tool (MCP-007): for stdio, command is the executable itself (required, non-empty) and
  args is a required literal list (it may be empty). Nothing is split on whitespace; a missing args or an
  empty command is a ToolError::Validation naming the field (McpStdioCommand::from_connect_args).
  Commands, arguments and environment values never appear in Debug output or daemon errors.
  """

  Background: User Story
    As a daemon client or agent launching a stdio MCP server
    I want to give the executable and its arguments as separate literal values
    So that arguments with spaces, empty arguments and paths with spaces reach the server unchanged

  Scenario: A daemon client's structured stdio arguments reach the server literally
    Given a token-armed daemon with an idle session A
    When a bearer-authenticated client configures stdio server "argv" with arguments "two words", "", "say \"hi\"" and "naïve ☃"
    Then calling "mcp__argv__search" in session A returns exactly those four arguments

  Scenario: An executable whose path contains a space is spawned literally
    Given a token-armed daemon with an idle session A and the fixture MCP server copied into a directory named "dir with space"
    When a bearer-authenticated client configures stdio server "spaced" with that executable and no arguments
    Then the response lists server "spaced" with its tool "search"

  Scenario: ConnectMCP with args passes the command and every argument literally
    Given a live MCP session and the ConnectMCP tool
    When the agent calls ConnectMCP for stdio server "argv" with the fixture as command and args "--argv", "a b" and ""
    Then calling "mcp__argv__search" returns exactly the arguments "a b" and ""

  Scenario: A missing stdio executable fails without revealing the command or its arguments
    Given a token-armed daemon with an idle session A
    When a bearer-authenticated client configures stdio server "missing" with executable "/nonexistent dir/mcp-secret-bin" and argument "--token=arg-secret"
    Then the request fails with an error naming "missing"
    And the error contains neither "mcp-secret-bin" nor "arg-secret"
    And session A has no MCP connections

  Scenario: Debug output of a stdio transport hides the command and its arguments
    Given a stdio session MCP transport with command "/opt/secret-bin" and argument "--token=arg-secret"
    When it is formatted with Debug
    Then the output contains neither "secret-bin" nor "arg-secret"

  Scenario: ConnectMCP stdio without args is rejected naming args
    Given a live MCP session and the ConnectMCP tool
    When the agent calls ConnectMCP for stdio server "split" with command "<fixture> --argv one two" and no args
    Then the call fails with a validation error naming "args"
    And the session has no MCP connections

  Scenario: ConnectMCP stdio with an empty command is rejected naming command
    Given a live MCP session and the ConnectMCP tool
    When the agent calls ConnectMCP for stdio server "blank" with command "   " and an empty args list
    Then the call fails with a validation error naming "command"
    And the session has no MCP connections
