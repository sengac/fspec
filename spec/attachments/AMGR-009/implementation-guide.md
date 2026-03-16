# AMGR-009 — Core AgentManager Tool (spawn, list, get_status, close)

## Summary

Implement the AgentManager tool module with the handler-delegated pattern (like SessionSearchTool) and the core lifecycle actions. After this story, agents can create subordinate workers, discover sessions, check status, and clean up.

## Actions

### spawn
- **Parameters:** role (optional string — system prompt overlay)
- **Behavior:** Creates a new subordinate session running regular agent_loop. Inherits supervisor's model. Registers spawner→spawned relationship in ChainOfCommand.
- **Response:** `{ session_id: "uuid" }`
- **Note:** Subordinate starts idle, waiting for a message. No initial prompt — the role (if set) preconditions the session, the first message triggers work.

### list
- **Parameters:** none
- **Response:** `{ sessions: [{ session_id, name, role, status, spawner_id, subordinate_ids }] }`
- **Note:** Returns all sessions visible in the project. No access control.

### get_status
- **Parameters:** session_id (required)
- **Response:** `{ session_id, role, status, model, spawner_id, subordinate_ids, pending_messages }`
- **Error:** `{ error: true, code: "session_not_found", message: "..." }`

### close
- **Parameters:** session_id (required)
- **Behavior:** Terminates subordinate session. Cleans up ChainOfCommand. Only the spawner or user can close.
- **Response:** `{ closed: true, session_id }`
- **Error:** `{ error: true, code: "permission_denied", message: "..." }`

## Implementation Pattern

Follow the handler-delegated pattern from SessionSearchTool:

```
codelet-tools/src/agent_manager/
├── mod.rs          # AgentManagerTool struct, impl Tool
├── handler.rs      # Handler registry (static HashMap<Uuid, Handler>)
├── types.rs        # AgentManagerAction, AgentManagerArgs, AgentManagerResult, SessionEntry
└── tests.rs        # Unit tests with mock handlers
```

Handler registered in session_manager.rs agent_loop() before each run, unregistered after.

## Provider Registration

Add `.tool(AgentManagerTool::new(session_id))` in all 5 providers' `create_rig_agent()`:
- claude.rs
- openai.rs
- gemini.rs (may need facade wrapper)
- zai.rs (may need facade wrapper)
- codex/mod.rs (may need facade wrapper)

## Response Shapes

All success responses are JSON objects with action-specific fields.
All error responses: `{ error: true, code: string, message: string }`
Error codes: session_not_found, permission_denied, invalid_parameter
