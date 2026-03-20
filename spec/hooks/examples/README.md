# Agent Lifecycle Hook Examples

Example hook scripts demonstrating all 6 agent lifecycle events. Copy and adapt these for your own projects.

## Quick Start

1. Copy `fspec-hooks.json` into your project's `spec/` directory
2. Copy the `.sh` scripts into `spec/hooks/examples/` (or wherever you prefer)
3. Make the scripts executable: `chmod +x spec/hooks/examples/*.sh`
4. Adjust the `command` paths in `fspec-hooks.json` to match your layout

## The 6 Agent Lifecycle Events

### 1. `session_start` — Session Initialization

**Script:** [`on-session-start.sh`](on-session-start.sh)  
**Format:** `HookDefinition[]` (name, command, blocking, timeout)  
**When:** Fires when an agent session starts (startup or resume)

**What it does:**
- Logs session start to `.fspec/hooks.log`
- Injects project coding standards as context via plain text stdout

**Context injection:** Plain text stdout is automatically injected as a system-level message. The agent sees your standards before processing any prompts.

```json
{
  "name": "inject-project-standards",
  "command": "spec/hooks/examples/on-session-start.sh",
  "blocking": false,
  "timeout": 10
}
```

---

### 2. `session_end` — Session Cleanup

**Script:** [`on-session-end.sh`](on-session-end.sh)  
**Format:** `HookDefinition[]`  
**When:** Fires when a session ends (completed, cancelled, exit, or error)

**What it does:**
- Logs session end with termination reason
- Template for Slack/webhook notifications (commented out)

**Note:** session_end hooks are fire-and-forget. They cannot block or inject context.

```json
{
  "name": "log-session-end",
  "command": "spec/hooks/examples/on-session-end.sh",
  "blocking": false,
  "timeout": 10
}
```

---

### 3. `user_prompt_submit` — Prompt Policy Enforcement

**Script:** [`on-user-prompt.sh`](on-user-prompt.sh)  
**Format:** `HookDefinition[]`  
**When:** Fires after the user submits a prompt, before the agent sees it

**What it does:**
- Blocks prompts attempting to override agent instructions
- Blocks prompts with destructive intent (`rm -rf`, `drop database`)
- Allows normal prompts through

**Blocking:** Exit code 2 + stderr message → prompt rejected, user sees the message. The agent never processes the blocked prompt.

```json
{
  "name": "policy-enforcement",
  "command": "spec/hooks/examples/on-user-prompt.sh",
  "blocking": true,
  "timeout": 5
}
```

---

### 4. `pre_tool_use` — Tool Call Security Gate

**Script:** [`on-pre-tool-use.sh`](on-pre-tool-use.sh)  
**Format:** `HookGroup[]` (with regex `matcher` for tool name filtering)  
**When:** Fires BEFORE a tool executes

**What it does:**
- **Denies** destructive system commands (`rm -rf /`, `mkfs`, `chmod 777`)
- **Asks** user for approval on network commands (`curl`, `wget`)
- **Continues** (no opinion) for safe commands

**Decisions (Claude Code compatible JSON):**
- `permissionDecision: "allow"` → auto-approve, skip permission prompt
- `permissionDecision: "deny"` → block the tool call
- `permissionDecision: "ask"` → force interactive user prompt
- Exit 0, no JSON → Continue (no opinion)

**Short-circuit:** Allow/Deny stops evaluation of remaining hook groups. Continue passes to the next group.

```json
{
  "matcher": "Bash",
  "hooks": [
    { "command": "spec/hooks/examples/on-pre-tool-use.sh", "timeout": 5 }
  ]
}
```

---

### 5. `post_tool_use` — Post-Execution Analysis

**Script:** [`on-post-tool-use.sh`](on-post-tool-use.sh)  
**Format:** `HookGroup[]` (with regex `matcher`)  
**When:** Fires AFTER a tool finishes executing

**What it does:**
- Matches only `Write` and `Edit` tool calls
- Template for running a linter and injecting results (commented out)

**Context injection:** Return JSON with `hookSpecificOutput.additionalContext` to inject lint warnings, test results, or other feedback as system messages.

```json
{
  "matcher": "Write|Edit",
  "hooks": [
    { "command": "spec/hooks/examples/on-post-tool-use.sh", "timeout": 10 }
  ]
}
```

---

### 6. `notification` — System Event Logger

**Script:** [`on-notification.sh`](on-notification.sh)  
**Format:** `HookDefinition[]`  
**When:** Fires when the agent system emits notifications (permission prompts, task completions, errors)

**What it does:**
- Logs notification type, title, and message to `.fspec/hooks.log`
- Template for desktop notifications via `osascript` (commented out)

```json
{
  "name": "log-notifications",
  "command": "spec/hooks/examples/on-notification.sh",
  "blocking": false,
  "timeout": 5
}
```

---

## JSON Payload Reference

Every hook receives a JSON payload on stdin and environment variables on the process:

### Environment Variables (all events)

| Variable | Description |
|----------|-------------|
| `FSPEC_PROJECT_DIR` | Workspace root path |
| `FSPEC_SESSION_ID` | UUID of the agent session |
| `FSPEC_HOOK_EVENT` | Event name (e.g., `PreToolUse`) |
| `FSPEC_TRANSCRIPT_PATH` | Path to the session transcript file |

### Payloads by Event

**session_start:**
```json
{ "hook_event_name": "SessionStart", "session_id": "...", "cwd": "...", "source": "startup|resume", "transcript_path": "..." }
```

**session_end:**
```json
{ "hook_event_name": "SessionEnd", "session_id": "...", "cwd": "...", "reason": "completed|cancelled|exit|error", "transcript_path": "..." }
```

**user_prompt_submit:**
```json
{ "hook_event_name": "UserPromptSubmit", "session_id": "...", "cwd": "...", "prompt": "the user's message", "transcript_path": "..." }
```

**pre_tool_use:**
```json
{ "hook_event_name": "PreToolUse", "session_id": "...", "cwd": "...", "tool_name": "Bash", "tool_input": {...}, "transcript_path": "..." }
```

**post_tool_use:**
```json
{ "hook_event_name": "PostToolUse", "session_id": "...", "cwd": "...", "tool_name": "Write", "tool_input": {...}, "tool_response": "...", "transcript_path": "..." }
```

**notification:**
```json
{ "hook_event_name": "Notification", "session_id": "...", "cwd": "...", "notification_type": "permission_prompt", "title": "...", "message": "...", "transcript_path": "..." }
```

## Exit Code Semantics

| Exit Code | Meaning |
|-----------|---------|
| `0` | Success / Continue (no opinion) |
| `2` + stderr | Deny / Block (reason = stderr message) |
| `2` + no stderr | Warning (continue) |
| Other non-zero | Non-blocking warning |
| Timeout | Deny for `pre_tool_use` (safety-first), Warning for all others |

## Claude Code Compatible JSON Response

Hook stdout can contain JSON compatible with Claude Code's hook protocol:

```json
{
  "continue": true,
  "decision": "allow",
  "reason": "Explanation text",
  "suppressOutput": false,
  "hookSpecificOutput": {
    "permissionDecision": "allow|deny|ask",
    "additionalContext": "Text injected as system message"
  }
}
```

## Config Format Reference

See [`fspec-hooks.json`](fspec-hooks.json) for a complete example with all 6 events configured. Key structure:

```json
{
  "global": {
    "timeout": 30,
    "shell": "bash -c"
  },
  "hooks": {
    "session_start":        [{ "name": "...", "command": "...", "blocking": false, "timeout": 10 }],
    "session_end":          [{ "name": "...", "command": "...", "blocking": false, "timeout": 10 }],
    "user_prompt_submit":   [{ "name": "...", "command": "...", "blocking": true,  "timeout": 5  }],
    "notification":         [{ "name": "...", "command": "...", "blocking": false, "timeout": 5  }],
    "pre_tool_use":         [{ "matcher": "Bash", "hooks": [{ "command": "...", "timeout": 5 }] }],
    "post_tool_use":        [{ "matcher": "Write|Edit", "hooks": [{ "command": "...", "timeout": 10 }] }]
  }
}
```

**Non-tool events** (`session_start`, `session_end`, `user_prompt_submit`, `notification`) use flat `HookDefinition[]` arrays.

**Tool events** (`pre_tool_use`, `post_tool_use`) use `HookGroup[]` arrays with optional regex `matcher` for tool name filtering.
