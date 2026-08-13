@TOOL-016
Feature: Unified Exec Tool with PTY Session Management
  """
  Create rust/tools/src/unified_exec/ module with mod.rs (constants, types), process_store.rs (ProcessStore HashMap with LRU eviction), and tool.rs (UnifiedExecTool implementing rig::tool::Tool). The tool uses action-based dispatch: run spawns processes, write/poll interact with running sessions, list/close manage sessions. ProcessStore is a global static behind Arc<Mutex<>>. BashTool remains unchanged for direct Claude usage; providers that need session management use UnifiedExecTool via facades.
  New facade trait ExecToolFacade in rust/tools/src/facade/traits.rs with InternalExecParams enum (Run, Write, Poll, List, Close variants). ExecToolFacadeWrapper in wrapper.rs delegates to UnifiedExecTool. Codex facades (BUG-114, BUG-115) implement ExecToolFacade to map exec_command/write_stdin/shell to the internal params. Current BashToolFacade and BashToolFacadeWrapper remain untouched.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The unified exec tool MUST support five actions: run, write, poll, list, close dispatched via an action parameter
  #   2. The run action spawns a process (PTY if tty=true, pipe otherwise), collects output for yield_time_ms, and returns exit_code if exited or session_id if still running
  #   3. The write action sends input bytes to a running session's stdin and polls for output, returning new output plus session status
  #   4. The poll action checks for new output from a running session without sending input, with a higher minimum wait time (5000ms)
  #   5. ProcessStore MUST enforce a max of 64 concurrent processes with LRU eviction protecting the 8 most recently used sessions
  #   6. Yield time MUST be clamped: min 250ms, max 30000ms, default 10000ms; poll/empty-write uses min 5000ms
  #   7. Output buffer per session MUST be capped at 1 MiB (UNIFIED_EXEC_OUTPUT_MAX_BYTES)
  #   8. When tty=false (default) and process exits within yield_time_ms, behavior MUST be identical to current BashTool one-shot execution for backward compatibility
  #   9. Background reaper tasks MUST watch for process exit and clean up ProcessStore entries automatically
  #   10. The command parameter MUST accept both string (shell command) and array of strings (argv for execvp-style) forms
  #   11. The tool MUST respect session isolation via effective_cwd lookup (TOOL-014 pattern) for workdir resolution
  #   12. The tool MUST check commands against the blocklist before execution (same as current BashTool)
  #
  # EXAMPLES:
  #   1. Agent runs 'ls -la' with default settings → process exits immediately → returns exit_code: 0, output: file listing, no session_id (backward compatible with BashTool)
  #   2. Agent runs 'python3' with tty=true → process stays running → returns session_id: 'abc123', no exit_code, output: Python prompt text
  #   3. Agent calls write action with session_id='abc123', input='print(42)\n' → returns output: '42\n', session_id still present (process alive)
  #   4. Agent calls write with input='exit()\n' → Python exits → returns exit_code: 0, no session_id, output includes exit message
  #   5. Agent calls poll on a running session → returns any new output since last read, session_id still present
  #   6. Agent calls list action → returns array of active sessions with their session_ids and metadata
  #   7. Agent calls close with session_id → process is killed, session removed from ProcessStore, returns confirmation
  #   8. 65th process spawned when store is full → LRU eviction kills least recently used session (not in top 8 recent), new process takes its slot
  #   9. Agent runs ['ls', '-la'] as argv array → execvp-style execution without shell interpretation (no glob expansion)
  #   10. Agent runs 'sleep 300' with yield_time_ms=2000 → after 2 seconds, returns session_id (process still running), output may be empty
  #   11. Agent runs command with yield_time_ms=50 → clamped to 250ms minimum, process given at least 250ms to produce output
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to execute commands with session management including PTY support and yield-and-resume patterns
    So that interactive long-running processes can be controlled across multiple tool calls

  # ========================================
  # Run Action — One-Shot Execution
  # ========================================
  @run
  @backward-compatible
  Scenario: Run a short-lived command returns exit_code and output
    Given the unified exec tool is available
    When I call the run action with command "echo hello"
    Then the response should contain exit_code 0
    And the response should contain output "hello"
    And the response should not contain a session_id

  @run
  @argv
  Scenario: Run command as argv array uses execvp without shell interpretation
    Given the unified exec tool is available
    When I call the run action with command as array ["ls", "-la"]
    Then the response should contain exit_code
    And the response should contain output with file listing
    And the response should not contain a session_id

  # ========================================
  # Run Action — Session Creation (Yield-and-Resume)
  # ========================================
  @run
  @session
  @pty
  Scenario: Run an interactive process with tty returns session_id
    Given the unified exec tool is available
    When I call the run action with command "cat" and tty true and yield_time_ms 500
    Then the response should contain a session_id
    And the response should not contain exit_code
    And the response should contain output

  @run
  @session
  @yield
  Scenario: Run a long-running command yields session_id after yield_time_ms
    Given the unified exec tool is available
    When I call the run action with command "sleep 300" and yield_time_ms 2000
    Then the response should contain a session_id after approximately 2 seconds
    And the response should not contain exit_code

  @run
  @yield
  @clamp
  Scenario: Yield time is clamped to minimum 250ms
    Given the unified exec tool is available
    When I call the run action with command "sleep 300" and yield_time_ms 50
    Then the yield_time_ms used should be at least 250ms
    And the response should contain a session_id

  @run
  @yield
  @clamp
  Scenario: Yield time is clamped to maximum 30000ms
    Given the unified exec tool is available
    When I call the run action with yield_time_ms 60000
    Then the yield_time_ms used should be at most 30000ms

  # ========================================
  # Write Action — Send Input to Running Session
  # ========================================
  @write
  @session
  Scenario: Write input to a running session and receive output
    Given a running session with session_id from command "cat" and tty true
    When I call the write action with that session_id and input "hello\n"
    Then the response should contain output with "hello"
    And the response should contain the session_id
    And the response should not contain exit_code

  @write
  @session
  @exit
  Scenario: Write causes process to exit returns exit_code
    Given a running session with session_id from command "cat" and tty true
    When I call the write action with EOF signal to terminate the process
    Then the response should contain exit_code
    And the response should not contain a session_id

  # ========================================
  # Poll Action — Check for Output
  # ========================================
  @poll
  @session
  Scenario: Poll a running session returns new output
    Given a running session with session_id that is producing output
    When I call the poll action with that session_id
    Then the response should contain any new output since last read
    And the response should contain the session_id

  @poll
  @yield
  Scenario: Poll uses higher minimum yield time of 5000ms
    Given a running session with session_id
    When I call the poll action with yield_time_ms 1000
    Then the effective yield_time_ms should be at least 5000ms

  # ========================================
  # List Action — Enumerate Active Sessions
  # ========================================
  @list
  Scenario: List active sessions returns session metadata
    Given there are 3 running sessions in the ProcessStore
    When I call the list action
    Then the response should contain 3 sessions
    And each session should have a session_id

  @list
  Scenario: List with no active sessions returns empty array
    Given there are no running sessions in the ProcessStore
    When I call the list action
    Then the response should contain 0 sessions

  # ========================================
  # Close Action — Terminate a Session
  # ========================================
  @close
  @session
  Scenario: Close a running session kills the process
    Given a running session with session_id
    When I call the close action with that session_id
    Then the process should be terminated
    And the session should be removed from ProcessStore
    And the response should confirm closure

  @close
  @error
  Scenario: Close with invalid session_id returns error
    Given the unified exec tool is available
    When I call the close action with session_id "nonexistent"
    Then the response should contain an error about unknown session

  # ========================================
  # ProcessStore — Capacity and LRU Eviction
  # ========================================
  @processstore
  @lru
  Scenario: LRU eviction when ProcessStore is full
    Given 64 running sessions in the ProcessStore
    When I call the run action to spawn a 65th process
    Then the least recently used session not in the 8 most recent should be evicted
    And the new process should be stored in its place

  @processstore
  @reaper
  Scenario: Background reaper cleans up exited processes
    Given a session whose process has exited
    When the background reaper task runs
    Then the session should be removed from ProcessStore

  @processstore
  @buffer
  Scenario: Output buffer capped at 1 MiB
    Given a running session producing output
    When the output exceeds 1 MiB
    Then the oldest output should be discarded to maintain the 1 MiB cap

  # ========================================
  # Integration — Blocklist and Session Isolation
  # ========================================
  @blocklist
  Scenario: Blocked command is rejected before execution
    Given the unified exec tool is available
    And the command "rm -rf /" is on the blocklist
    When I call the run action with command "rm -rf /"
    Then the command should be rejected with a blocklist error
    And no process should be spawned

  @isolation
  Scenario: Session isolation uses effective_cwd for workdir
    Given a session with effective_cwd set to "/tmp/worktree"
    When I call the run action with command "pwd"
    Then the command should execute in "/tmp/worktree"
    And the output should contain "/tmp/worktree"

  @workdir
  Scenario: Explicit workdir overrides default but not session isolation
    Given the unified exec tool is available
    When I call the run action with command "pwd" and workdir "/tmp"
    Then the command should execute in "/tmp"
