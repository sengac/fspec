# Agent Lifecycle Hook Examples

Example hook scripts demonstrating all 6 agent lifecycle events. Copy and adapt these for your own projects.

## Quick Start

1. Copy `fspec-hooks.json.example` to `spec/fspec-hooks.json` in your project root
2. Copy the `.sh` scripts to `spec/hooks/` (or wherever you prefer)
3. Make the scripts executable: `chmod +x spec/hooks/*.sh`
4. Adjust the `command` paths in `spec/fspec-hooks.json` to match your layout

```bash
cp examples/hooks/fspec-hooks.json.example spec/fspec-hooks.json
mkdir -p spec/hooks
cp examples/hooks/*.sh spec/hooks/
chmod +x spec/hooks/*.sh
```

### Two-Level Config

Hooks can be configured at two levels, which are concatenated at runtime:

| Level | Path | Priority |
|-------|------|----------|
| User | `~/.fspec/fspec-hooks.json` | First (runs before project-level) |
| Project | `spec/fspec-hooks.json` | Appended after user-level |

Both files use the same JSON format. Project-level `global` settings take precedence over user-level.

---

## The 6 Agent Lifecycle Events

### 1. `session_start` — Session Initialization

**Script:** [`on-session-start.sh`](on-session-start.sh)
**Format:** `HookDefinition[]` (name, command, timeout)
**When:** Fires when an agent session starts (startup or resume)

**What it does:**
- Logs session start to `.fspec/hooks.log`
- Injects project coding standards as context via plain text stdout

**Context injection:** Plain text stdout is automatically injected as a system-level message. The agent sees your standards before processing any prompts.

```json
{
  "name": "inject-project-standards",
  "command": "spec/hooks/on-session-start.sh",
  "timeout": 10
}
```

**Blocking:** Non-blocking. Exit codes produce warnings only — the session always starts.

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
  "command": "spec/hooks/on-session-end.sh",
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

**Blocking:** Exit code 2 + stderr message → prompt blocked, user sees the message. The agent never processes the blocked prompt.

```json
{
  "name": "policy-enforcement",
  "command": "spec/hooks/on-user-prompt.sh",
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
- `hookSpecificOutput.permissionDecision: "allow"` → auto-approve, skip permission prompt
- `hookSpecificOutput.permissionDecision: "deny"` → block the tool call
- `hookSpecificOutput.permissionDecision: "ask"` → force interactive user prompt
- Exit 0, no JSON → Continue (no opinion)

**Short-circuit:** Allow/Deny stops evaluation of remaining hook groups. Continue passes to the next group.

**Timeout:** If a pre_tool_use hook times out, the tool call is **denied** (safety-first).

```json
{
  "matcher": "Bash",
  "hooks": [
    { "command": "spec/hooks/on-pre-tool-use.sh", "timeout": 5 }
  ]
}
```

**Matcher:** The `matcher` is a regex that must match the **entire** tool name (anchored as `^(?:PATTERN)$`). For example, `"Bash"` matches only the tool named exactly "Bash".

---

### 5. `post_tool_use` — Post-Execution Analysis

**Script:** [`on-post-tool-use.sh`](on-post-tool-use.sh)
**Format:** `HookGroup[]` (with regex `matcher`)
**When:** Fires AFTER a tool finishes executing

**What it does:**
- Matches only `Write` and `Edit` tool calls
- Template for running a linter and injecting results (commented out)

**Context injection:** Return JSON with `hookSpecificOutput.additionalContext` to inject lint warnings, test results, or other feedback as system messages.

**Timeout:** If a post_tool_use hook times out, a warning is logged but execution continues.

```json
{
  "matcher": "Write|Edit",
  "hooks": [
    { "command": "spec/hooks/on-post-tool-use.sh", "timeout": 10 }
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
  "command": "spec/hooks/on-notification.sh",
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
| `FSPEC_HOOK_EVENT` | Event name (e.g., `SessionStart`) |
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

| Exit Code | session_start / session_end / notification | user_prompt_submit | pre_tool_use |
|-----------|-------------------------------------------|--------------------|------------------------|
| `0` | Success | Allow (unless JSON says otherwise) | Continue (no opinion) |
| `2` + stderr | Warning (logged) | **Block** prompt | **Deny** tool call |
| `2` + no stderr | Warning | Warning | Warning (continue) |
| Other non-zero | Warning | Warning | Warning (continue) |
| Timeout | Warning | Warning | **Deny** (safety-first) |

For `post_tool_use`: all non-zero exit codes produce warnings only (never blocks).

## Claude Code Compatible JSON Response

Hook stdout can contain JSON compatible with Claude Code's hook protocol:

```json
{
  "continue": true,
  "decision": "allow",
  "reason": "Explanation text",
  "hookSpecificOutput": {
    "permissionDecision": "allow|deny|ask",
    "additionalContext": "Text injected as system message"
  }
}
```

**JSON response priority for pre_tool_use:**
1. `hookSpecificOutput.permissionDecision` → Allow/Deny/Ask
2. `continue: false` → Deny
3. `decision: "deny"` or `"block"` → Deny
4. Exit code 2 + stderr → Deny
5. Everything else → Continue (no opinion)

## Config Format Reference

See [`fspec-hooks.json.example`](fspec-hooks.json.example) for a complete example with all 6 events configured. Key structure:

```json
{
  "global": {
    "timeout": 30,
    "shell": "bash -c"
  },
  "hooks": {
    "session_start":        [{ "name": "...", "command": "...", "timeout": 10 }],
    "session_end":          [{ "name": "...", "command": "...", "timeout": 10 }],
    "user_prompt_submit":   [{ "name": "...", "command": "...", "timeout": 5  }],
    "notification":         [{ "name": "...", "command": "...", "timeout": 5  }],
    "pre_tool_use":         [{ "matcher": "Bash", "hooks": [{ "command": "...", "timeout": 5 }] }],
    "post_tool_use":        [{ "matcher": "Write|Edit", "hooks": [{ "command": "...", "timeout": 10 }] }]
  }
}
```

**Non-tool events** (`session_start`, `session_end`, `user_prompt_submit`, `notification`) use flat `HookDefinition[]` arrays.

**Tool events** (`pre_tool_use`, `post_tool_use`) use `HookGroup[]` arrays with optional regex `matcher` for tool name filtering.
