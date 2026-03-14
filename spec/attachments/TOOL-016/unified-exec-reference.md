# TOOL-016: Unified Exec Tool with PTY Session Management — Reference Architecture

## Overview

A provider-agnostic exec tool that replaces one-shot BashTool with session-aware execution. Follows VTCode's `unified_exec` pattern and supports the yield-and-resume lifecycle that Codex's `exec_command`/`write_stdin` rely on.

## Action Dispatch

Single tool with `action` parameter:

| Action | Description | Required params |
|--------|-------------|----------------|
| `run` | Execute command (default) | `command` |
| `write` | Send stdin to running session | `session_id`, `input` |
| `poll` | Read output from running session | `session_id` |
| `list` | List active sessions | — |
| `close` | Terminate a session | `session_id` |

## Schema

```json
{
  "type": "object",
  "properties": {
    "action": {
      "type": "string",
      "enum": ["run", "write", "poll", "list", "close"],
      "description": "Action. Inferred from command/input/session_id when omitted."
    },
    "command": {
      "anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}],
      "description": "Command as shell string or argv array."
    },
    "input": {"type": "string", "description": "stdin content for write action."},
    "session_id": {"type": "string", "description": "Session ID for write/poll/close."},
    "workdir": {"type": "string", "description": "Working directory."},
    "tty": {"type": "boolean", "description": "Allocate PTY (default: false)."},
    "yield_time_ms": {"type": "integer", "description": "Wait time for output (ms). Default: 10000."},
    "max_output_tokens": {"type": "integer", "description": "Max output tokens."},
    "timeout_secs": {"type": "integer", "description": "Hard timeout in seconds. Default: 120."}
  }
}
```

## Response Schema

```json
{
  "session_id": "string — present when process is still running",
  "exit_code": "integer — present when process has exited",
  "output": "string — command output, possibly truncated",
  "wall_time_seconds": "number"
}
```

## Yield-and-Resume Pattern (from Codex reference)

### How it works

The LLM never communicates mid-tool-call. Instead:

1. **`run` action starts a process:**
   - Spawns process (PTY if `tty=true`, pipe otherwise)
   - Collects output for `yield_time_ms` (clamped to 250ms–30s)
   - If process **exits** → returns `exit_code`, cleans up
   - If process **still running** → stores in ProcessStore, returns `session_id`

2. **`write` action sends input:**
   - Looks up session in ProcessStore
   - Sends bytes to stdin via mpsc channel
   - Sleeps 100ms for process to react
   - Polls output for `yield_time_ms`
   - Returns new output + session status

3. **`poll` action checks for output:**
   - Like `write` but with no stdin input
   - Higher minimum wait (5000ms) since we're just checking
   - Used for long-running processes

### ProcessStore Design

```rust
struct ProcessStore {
    processes: HashMap<String, ProcessEntry>,
}

struct ProcessEntry {
    child: tokio::process::Child,  // or PTY handle
    stdin_tx: mpsc::Sender<Vec<u8>>,
    output_buffer: Arc<Mutex<Vec<u8>>>,
    output_notify: Arc<Notify>,
    last_used: Instant,
    tty: bool,
}
```

Key constraints (from Codex reference):
- **Max processes**: 64 (`MAX_UNIFIED_EXEC_PROCESSES`)
- **LRU eviction**: When full, evict least-recently-used (protect 8 most recent)
- **Background reaper**: Spawned tasks watch for process exit and clean up
- **Output buffer**: 1 MiB max (`UNIFIED_EXEC_OUTPUT_MAX_BYTES`)

### Yield Time Constants

```rust
const MIN_YIELD_TIME_MS: u64 = 250;
const MIN_EMPTY_YIELD_TIME_MS: u64 = 5_000;  // for poll/empty write
const MAX_YIELD_TIME_MS: u64 = 30_000;
const DEFAULT_YIELD_TIME_MS: u64 = 10_000;
```

## Backward Compatibility

The existing BashTool behavior (one-shot execution) is equivalent to:
```json
{"action": "run", "command": "ls -la", "tty": false}
```

When `tty=false` (default) and the process exits within `yield_time_ms`, the behavior is identical to current BashTool — command runs, output returned, no session management.

## VTCode Reference Files

- `vtcode-core/src/tools/registry/builtins.rs` — registration with all aliases
- `vtcode-core/src/tools/handlers/session_tool_catalog.rs` — `unified_exec_parameters()` schema
- `vtcode-core/src/tools/registry/executors.rs` — `execute_unified_exec_internal()`
- `vtcode-config/src/constants/tools.rs` — tool name constants and legacy aliases

## Codex Reference Files

- `codex-rs/core/src/tools/spec.rs` — tool schemas (`create_exec_command_tool()`, `create_write_stdin_tool()`, `create_shell_tool()`)
- `codex-rs/core/src/tools/handlers/unified_exec.rs` — handler with exec_command/write_stdin dispatch
- `codex-rs/core/src/unified_exec/process_manager.rs` — ProcessStore, collect_output_until_deadline, LRU pruning
- `codex-rs/core/src/unified_exec/mod.rs` — constants, ExecCommandRequest, WriteStdinRequest
- `codex-rs/core/src/tools/context.rs` — ExecCommandToolOutput, response formatting
