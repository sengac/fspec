@BUG-582
@done
@TOOL-027
@tools
@session
@rpc
@tool-execution
Feature: Send an interrupt or end of input to a running exec session
  """
  New FspecService method control_exec_stdin(session_id: SessionId, exec_session_id: String, control: ExecStdinControl) -> Result<(), String>, appended after configure_session_mcp_servers (wire_order_frozen.txt gains it last). ExecStdinControl (rpc-types) = Interrupt (0) | Eof (1). Layers: rpc-types; rpc service_declaration/session_mcp.rs + service_implementation (forwards to SessionManagerHandle); mcp_dispatch group_43 (catalog, validate, dispatch); SessionManagerHandle::control_exec_stdin (default refuses); sessions handle impl (unknown agent session -> 'Session not found'; a success clears the stored exec-stdin request like write_exec_stdin, BUG-171); unified_exec ProcessStore::control_stdin in stdin_control.rs. Pipe session: Interrupt runs kill -INT against the process group (the pipe spawn uses process_group(0), so the shell pid is the pgid; no unsafe code); Eof replaces the entry's stdin sender with a closed one, so the forwarder task ends and drops the child's stdin, and later writes fail. PTY session: Interrupt writes 0x03 and Eof writes 0x04 through the PTY stdin channel. Windows pipe Interrupt returns an explicit not-supported error. FspecBackend gains control_exec_stdin (embedded and websocket transports and every test mock).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Interrupt delivers SIGINT to the exec session's command only; the agent session and its turn are not interrupted
  #   2. Pipe sessions: interrupt signals the command's process group with SIGINT; end of input closes the command's stdin, and later writes fail with an error naming the exec session
  #   3. PTY sessions: interrupt writes the terminal interrupt character (0x03) and end of input writes the end-of-file character (0x04), so the line discipline delivers them
  #   4. A successful control clears the agent session's pending exec-stdin request, the same as write_exec_stdin
  #   5. Unknown agent session returns 'Session not found'; an unknown or exited exec session returns an error naming the exec session id; interrupting a pipe session on Windows returns an explicit not-supported error
  #   6. control_exec_stdin is appended as the last FspecService method so every existing variant index stays stable
  #
  # EXAMPLES:
  #   1. A pipe session runs a trap script; interrupt makes it print got-int and exit
  #   2. A PTY session runs cat; end of input makes cat exit 0
  #   3. Interrupt for exec-missing fails with an error naming exec-missing
  #
  # ========================================
  Background: User Story
    As a TUI user answering a running command
    I want to send the command an interrupt or end of input
    So that I can stop or finish a command that waits on stdin without interrupting the agent

  Scenario: Interrupt sends SIGINT to a pipe exec session's command
    Given a daemon client and an agent session with a live pipe exec session that prints "got-int" on SIGINT
    When the client sends an interrupt to that exec session over the RPC
    Then the exec session prints "got-int" and exits
    And the agent session is still idle

  Scenario: Interrupt reaches a PTY exec session's foreground command
    Given a daemon client and an agent session with a live PTY exec session that prints "got-int" on SIGINT
    When the client sends an interrupt to that exec session over the RPC
    Then the exec session prints "got-int" and exits

  Scenario: End of input closes a pipe exec session's stdin
    Given a daemon client and an agent session with a live pipe exec session running "cat"
    When the client sends end of input to that exec session over the RPC
    Then the exec session exits with status 0
    And a later write to that exec session fails with an error naming the exec session

  Scenario: End of input ends a PTY exec session's input
    Given a daemon client and an agent session with a live PTY exec session running "cat"
    When the client sends end of input to that exec session over the RPC
    Then the exec session exits

  Scenario: A successful control clears the pending exec-stdin request
    Given a daemon client and an agent session with a pending exec-stdin request for a live pipe exec session running "cat"
    When the client sends end of input to that exec session over the RPC
    Then the agent session has no exec-stdin request

  Scenario: Control on an unknown exec session or agent session fails with a clean error
    Given a daemon client and an agent session with no exec session "exec-missing"
    When the client sends an interrupt to exec session "exec-missing" over the RPC
    Then the call fails with an error naming "exec-missing"
    And an interrupt for an unknown agent session fails with "Session not found"

  Scenario: control_exec_stdin is appended after every existing FspecService method
    Given the FspecService methods in declaration order
    When the index of control_exec_stdin is read
    Then control_exec_stdin keeps index 130, after every method declared before it
