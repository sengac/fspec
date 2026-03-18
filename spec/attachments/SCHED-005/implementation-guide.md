# SCHED-005: Shell Job Execution — Implementation Guide

## Overview

When the scheduler engine (SCHED-003) determines a shell-type schedule should fire, execute the configured command via the Bash tool pattern in the project directory. Record execution in session history for SessionSearch discoverability and update the last-run timestamp in `spec/schedules.json`.

## How Shell Execution Works Today

### The `BashTool` (codelet/tools/src/bash.rs)

The Bash tool uses `unified_exec` — a sophisticated process management system in `codelet/napi/src/unified_exec/`:

1. **`handle_exec()`** — Primary execution function
2. Spawns a child process via `tokio::process::Command`
3. Captures stdout/stderr with streaming progress via `StreamChunk::ToolProgress`
4. Supports background processes, interactive shells, and timeout handling
5. Returns stdout/stderr as the tool result

### Process Management (unified_exec)

Key files:
- `codelet/napi/src/unified_exec/mod.rs` — Main execution logic
- `codelet/napi/src/unified_exec/store.rs` — `ProcessStore` for tracking background processes
- `codelet/napi/src/unified_exec/reaper.rs` — Background process cleanup (the pattern SCHED-003 follows)

### For Scheduled Shell Jobs

We don't need the full `BashTool` + agent loop overhead for a simple shell command. Options:

**Option A: Spawn a lightweight session (Recommended)**

Create a minimal agent session that executes a single Bash tool call and exits. This keeps shell jobs in the same session infrastructure as agent jobs — unified logging, SessionSearch, session list visibility.

```rust
async fn trigger_shell_job(
    name: &str,
    schedule: &ScheduleEntry,
    project_path: &str,
) -> Result<ShellJobResult> {
    // 1. Spawn a minimal session (with schedule_triggered=true)
    // 2. Instead of sending a prompt to the LLM, directly execute the command
    // 3. Record the execution as session history (tool call + tool result chunks)
    // 4. Close the session
    // 5. Return exit code, stdout, stderr
}
```

**Option B: Direct process spawn (Simpler)**

Use `tokio::process::Command` directly without a session:

```rust
async fn trigger_shell_job(
    name: &str,
    command: &str,
    project_path: &str,
) -> Result<ShellJobResult> {
    let output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(project_path)
        .output()
        .await?;
    
    ShellJobResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    }
}
```

Option A is preferred because it gives us SessionSearch discoverability for free. Option B is simpler but requires a separate logging mechanism.

## Session History for Shell Jobs

For SessionSearch to find shell job results, we need to emit StreamChunks:

```rust
// Before execution
session.handle_output(StreamChunk::tool_call(ToolCallInfo {
    id: tool_call_id,
    name: "Bash".to_string(),
    input: json!({ "command": command }).to_string(),
}));

// During execution (streaming)
session.handle_output(StreamChunk::tool_progress(ToolProgressInfo {
    tool_call_id: tool_call_id.clone(),
    tool_name: "Bash".to_string(),
    output_chunk: stdout_line,
    is_stderr: false,
}));

// After execution
session.handle_output(StreamChunk::tool_result(ToolResultInfo {
    tool_call_id,
    content: format!("Exit code: {}\n\nStdout:\n{}\n\nStderr:\n{}", exit_code, stdout, stderr),
    is_error: exit_code != 0,
}));
```

## Exit Code Handling

| Exit Code | Action |
|-----------|--------|
| 0 | Success — update `lastRunStatus: "completed"` |
| Non-zero | Failure — update `lastRunStatus: "failed"`, emit failure notification (SCHED-010) |

## Files to Create/Modify

| File | Action | Description |
|------|--------|-------------|
| `codelet/napi/src/scheduler/shell_job.rs` | Create | Shell job execution logic |
| `codelet/napi/src/scheduler/types.rs` | Modify | Add `ShellJobResult` struct |

## Key Constraints

- Shell commands run in the **project directory** (cwd = the project path from the schedule)
- Commands execute via `sh -c "command"` (macOS/Linux) — cross-platform shell handling follows the same pattern as BashTool
- No execution timeout — the command runs to completion (consistent with SCHED-001 rules)
- stdout/stderr must be captured for bridge notifications (SCHED-010) and SessionSearch
- The shell environment should inherit the user's environment (PATH, etc.)
- After completion, `lastRunAt` and `lastRunStatus` are updated in spec/schedules.json
