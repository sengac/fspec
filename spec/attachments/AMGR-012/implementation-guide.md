# AMGR-012 — Role Management (set_role + /role TUI command)

## Summary

Implement the set_role action for AgentManager and the /role TUI command. A role is a simple string that acts as a system prompt overlay for a session. Any session can have a role — not just spawned ones.

## Action: set_role

### Parameters
- `session_id` (optional — defaults to caller's own session)
- `role` (required — string, the role text)

### Behavior
- Sets or replaces the role on the target session
- Role is injected as a system prompt overlay (prepended to the system prompt or injected as a system-reminder)
- Setting role to empty string or null clears the role
- Any session can have a role set — the caller's own, a subordinate's, or any session by ID

### Response
- Success: `{ session_id: "target-id", role: "new role text" }`
- Clear: `{ session_id: "target-id", role: null }`
- Error: `{ error: true, code: "session_not_found", message: "..." }`

## /role TUI Command

- User types `/role` in the TUI
- A dialog pops up with a text input for the role string
- Pre-populated with current role if one exists
- Submit sets the role on the current session
- Cancel/Esc dismisses without change
- Replaces the old `/supervisor` command

## How Role is Applied

The role string is stored on the BackgroundSession. When the agent_loop builds the next prompt, the role is included as part of the system context — similar to how CLAUDE.md or work unit reminders are injected. The exact injection mechanism (system-reminder tag, prepend to preamble, etc.) follows existing patterns.

## Scenarios

1. Set role on own session via set_role action
2. Set role on subordinate via set_role action
3. Clear role by setting empty string
4. /role command opens dialog in TUI
5. /role dialog pre-populates existing role
6. Role appears in get_status and list responses
7. set_role on non-existent session returns error
