# AMGR-003 — Core Tool Infrastructure + Agent Lifecycle

## Summary

Build the AgentManager tool module and implement all agent lifecycle actions: spawn, list, get_status, set_role, close. This is the minimum viable AgentManager — after this story, agents can create, discover, inspect, modify, and terminate supervisor sessions programmatically via tool calls.

## Depends On
- **WATCH-024** (DONE ✅) — supervisor/subordinate terminology

## Scenarios (12)

### Spawn (2)
1. **Spawn a supervisor session with role and brief** — spawn creates a new supervisor using existing infrastructure (ChainOfCommand, broadcast, supervisor_agent_loop). Model inherited from current session. Returns `{ session_id, role, brief }`.
2. **Spawned supervisor and subordinate are mutually aware** — on spawn, supervisor receives subordinate's session ID in initial context, subordinate receives notification with supervisor's session ID and role. Both can immediately use SessionSearch and message.

### List (2)
3. **List all sessions in the project** — returns all sessions with IDs, names, roles, status, subordinate/supervisor relationships, supervisor counts.
4. **Session IDs from list are usable as coordination handles** — returned IDs work directly with SessionSearch(show/search) and message(session_id).

### Get Status (1)
5. **Get detailed status of a specific session** — returns role, brief, auto_inject, subordinate ID, supervisor IDs, current state (idle/processing), model, pending message count.

### Set Role (1)
6. **Change an existing supervisor's role mid-session** — updates role, brief, auto_inject without restarting. Returns `{ session_id, role, brief, auto_inject, previous_role }`.

### Close (2)
7. **Close a session the agent spawned** — terminates session, cleans up ChainOfCommand and broadcast subscriptions. Returns `{ closed: true, session_id, role }`.
8. **Close is denied for sessions the agent did not spawn** — returns `{ error: true, code: 'permission_denied' }`. Session remains running.

### Error Handling (1)
9. **Consistent error response format** — all actions return `{ error: true, code, message }` on failure. Codes: session_not_found, permission_denied, invalid_parameter, delivery_failed.

### Tool Registration (2)
10. **Registered in all providers** — NAPI handler alongside SessionSearch and DeepSearch in every create_rig_agent().
11. **Independent from SessionSearch** — neither imports the other. Agents compose by passing session IDs.

### Mutual Awareness (1 — covered by spawn scenario 2)

## Applicable Rules
0, 7, 9, 11 (updated), 12, 13, 15, 16, 17, 18, 22, 23, 24, 28, 32, 34

## Key Implementation Details

### Tool Module
- Location: `codelet/tools/src/agent_manager.rs` (or `agent_manager/` directory)
- Action dispatch via match on `action_type` string parameter
- Same pattern as `session_search/` and `bridge.rs`

### Spawn Parameters
- `role`: string (required) — role name for the supervisor
- `brief`: string (required) — instructions for the supervisor
- `auto_inject`: boolean (default: true) — whether to auto-inject observations

### Infrastructure Reuse
- `ChainOfCommand` HashMap for subordinate→supervisors mapping
- `broadcast::channel(256)` for observation streaming
- `ObservationBuffer` with breakpoint detection
- `supervisor_agent_loop` with biased `tokio::select!`
- `SupervisorInput` mpsc injection channel

### Spawn Response
```json
{ "session_id": "uuid", "role": "security-reviewer", "brief": "Review for vulnerabilities..." }
```

### Error Response
```json
{ "error": true, "code": "session_not_found", "message": "No session found with ID abc-123" }
```

## Estimate: 8 points
Major infrastructure: new tool module, NAPI handler, 5 actions, reuse supervisor infrastructure, registration in all providers.
