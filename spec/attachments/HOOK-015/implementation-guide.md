# HOOK-015: Hook Execution Engine & Output Interpretation

## What This Card Delivers

The runtime engine that executes hook commands as child processes and interprets their output. After this card is complete, the Rust codebase can:
- Execute shell commands via `sh -c` with JSON payload on stdin
- Set FSPEC_* environment variables on hook processes
- Enforce timeouts with process kill
- Serialize per-event JSON payloads (SessionStart, PreToolUse, etc.)
- Interpret exit codes (0/2/other) into outcomes
- Parse Claude Code compatible JSON responses
- Extract additional context for conversation injection

## Depends On

- **HOOK-014** — config types, compiled hooks, matcher

## Command Execution

### Shell Execution
```rust
tokio::process::Command::new("sh")
    .arg("-c")
    .arg(&command)
    .current_dir(&workspace)
    .env("FSPEC_PROJECT_DIR", &workspace)
    .env("FSPEC_SESSION_ID", &session_id)
    .env("FSPEC_HOOK_EVENT", &event_name)
    .env("FSPEC_TRANSCRIPT_PATH", &transcript_path)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
```

### JSON Payload → stdin
Serialize a per-event payload struct and write to the child's stdin.

### Timeout
- Use `tokio::time::timeout(duration, child.wait_with_output())`
- On timeout: `child.start_kill()` then `child.wait()`
- Default timeout from `global.timeout` or 60s
- Per-hook timeout overrides global

## JSON Payloads (Per Event Type)

```rust
struct SessionStartPayload {
    hook_event_name: String,    // "SessionStart"
    session_id: String,
    cwd: String,
    source: String,             // "startup" or "resume"
    transcript_path: String,
}

struct SessionEndPayload {
    hook_event_name: String,    // "SessionEnd"
    session_id: String,
    cwd: String,
    reason: String,             // "completed", "exit", "cancelled", "error"
    transcript_path: String,
}

struct UserPromptSubmitPayload {
    hook_event_name: String,    // "UserPromptSubmit"
    session_id: String,
    cwd: String,
    prompt: String,
    transcript_path: String,
}

struct PreToolUsePayload {
    hook_event_name: String,    // "PreToolUse"
    session_id: String,
    cwd: String,
    tool_name: String,
    tool_input: serde_json::Value,
    transcript_path: String,
}

struct PostToolUsePayload {
    hook_event_name: String,    // "PostToolUse"
    session_id: String,
    cwd: String,
    tool_name: String,
    tool_input: serde_json::Value,
    tool_response: String,
    transcript_path: String,
}

struct NotificationPayload {
    hook_event_name: String,    // "Notification"
    session_id: String,
    cwd: String,
    notification_type: String,  // "permission_prompt", "idle_prompt"
    title: String,
    message: String,
    transcript_path: String,
}
```

## Exit Code Interpretation

| Exit Code | Stderr | Outcome |
|-----------|--------|---------|
| 0 | any | Success / Continue |
| 2 | non-empty | **Deny/Block** (stderr is the reason) |
| 2 | empty | Warning (continue) |
| timeout | n/a | Deny for pre_tool_use, Warning for others |
| 1, 3, etc. | any | Non-blocking warning |

## JSON Response Parsing (Claude Code Compatible)

Hook stdout may contain JSON. Parse it and extract these fields:

```json
{
  "continue": true,                    // false → deny/block
  "suppressOutput": false,             // suppress hook stdout from display
  "decision": "allow",                 // "allow", "deny", "block"
  "reason": "Approved by policy",      // human-readable explanation
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",     // "allow", "deny", "ask" (pre_tool_use only)
    "additionalContext": "Remember X"  // injected as system message
  }
}
```

### Interpretation Priority for pre_tool_use:
1. `hookSpecificOutput.permissionDecision` → Allow/Deny/Ask
2. `continue: false` → Deny
3. `decision: "deny"` or `"block"` → Deny
4. Exit code 2 + stderr → Deny
5. Everything else → Continue (no opinion)

### Context Injection:
- `hookSpecificOutput.additionalContext` → inject as system message
- For session_start/user_prompt_submit: plain text stdout (when not JSON) → inject as additional context

## Outcome Types

```rust
enum PreToolHookDecision { Continue, Allow, Deny, Ask }

struct HookMessage {
    level: HookMessageLevel,
    content: String,
}

enum HookMessageLevel { Info, Warning, Error }

struct SessionStartOutcome {
    messages: Vec<HookMessage>,
    additional_context: Vec<String>,
}

struct UserPromptOutcome {
    allow_prompt: bool,
    block_reason: Option<String>,
    additional_context: Vec<String>,
    messages: Vec<HookMessage>,
}

struct PreToolOutcome {
    decision: PreToolHookDecision,
    reason: Option<String>,
    messages: Vec<HookMessage>,
}

struct PostToolOutcome {
    block_reason: Option<String>,
    additional_context: Vec<String>,
    messages: Vec<HookMessage>,
}
```

## Scenarios (19)

All tagged `@HOOK-015` in `spec/features/agent-lifecycle-hooks.feature`:
- Command Execution (5): JSON payload on stdin, env vars, per-event payloads
- Timeout Handling (3): kill on timeout, deny vs warning by event type
- Exit Code Interpretation (4): code 0, 2+stderr, 2-no-stderr, other
- JSON Response (7): permissionDecision (allow/deny/ask), continue:false, additionalContext, plain text
