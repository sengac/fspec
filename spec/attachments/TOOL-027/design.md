# TOOL-027 design

TUI-116 needs Ctrl+C (SIGINT to the command only) and Ctrl+D (end of input) in the stdin answer box. The backend had only `write_exec_stdin`, which always appends a newline, so control bytes and signals had no path.

## Wire
- rpc-types `ExecStdinControl { Interrupt, Eof }` (serde, schemars, Copy). Bincode encodes it as variant index 0 or 1.
- `FspecService::control_exec_stdin(session_id: SessionId, exec_session_id: String, control: ExecStdinControl) -> Result<(), String>`, appended after `configure_session_mcp_servers` at variant index 130. wire_order_frozen.txt gains `control_exec_stdin` last.

## Behaviour
- `ProcessStore::control_stdin(exec_session_id, control)` (tools `unified_exec/stdin_control.rs`):
  - No entry: `Unknown or exited exec session: <id>`.
  - PTY: send `[0x03]` or `[0x04]` on the entry's stdin channel. A closed channel is an error naming the session.
  - Pipe Interrupt: `child.id()` (None means exited, which is an error) is the pgid. Run `kill -INT -- -<pgid>` and report a non-zero exit or a spawn failure. There is no `unsafe` and no allow. On Windows it is an explicit "not supported" error.
  - Pipe Eof: replace `entry.stdin_tx` with a sender whose receiver is already dropped. The store's real sender drops, the forwarder loop ends and drops the child's stdin (EOF), and later sends fail. No other long-lived sender exists.
- `SessionManagerHandle::control_exec_stdin` defaults to a refusal (like configure_session_mcp_servers). The sessions impl returns `Session not found: <id>` for an unknown agent session, runs the store op through `loop_block_on`, and on success clears the stored exec-stdin request (BUG-171 parity with write).
- rpc service impl forwards to the handle; without a handle it errors. MCP dispatch adds group_43 (destructive, not read-only).
- FspecBackend gains a required `control_exec_stdin`. Websocket and embedded call the RPC. Test mocks record calls, and MockBackend can script an error (used by TUI-116).

## Tests
One file, `rust/rpc-server/tests/tool027_exec_stdin_control.rs`, with helpers in `tool027_exec_stdin_control_support/mod.rs`. A real daemon over WebSocket with a real SessionManager: exec sessions are started in-process with `UnifiedExecTool::new(agent_uuid)` (same process, same global store), and the RPC client sends the control. The MCP-003 "is last" wire test changes to "keeps index 129".

## Not in scope
The write forwarders drop io errors (BUG-362). This card's PTY control send reports only channel delivery, the same as write_exec_stdin today.
