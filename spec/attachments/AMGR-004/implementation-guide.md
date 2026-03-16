# AMGR-004 — Agent Messaging — Plain, Bidirectional, and Any-to-Any

## Summary

Implement the `message` action for AgentManager enabling directed text communication between any two sessions regardless of ChainOfCommand relationship. Messages queue in an mpsc channel (capacity 16) and are picked up on the next `tokio::select!` iteration — no interruption of LLM generation.

## Depends On
- **AMGR-003** — tool module, lifecycle actions, session infrastructure

## Scenarios (7)

### Plain Message (2)
1. **Send a plain text message to another agent** — message delivered, returns `{ delivered: true, session_id, context_resolved: 0 }`.
2. **Plain message with no context array is valid** — context is optional. No resolution occurs.

### Queuing (1)
3. **Messages queue when target is processing** — message waits in mpsc channel, picked up after current turn. No interruption.

### Delivery Failure (1)
4. **Message delivery fails when channel is full** — 16 pending messages → `{ error: true, code: 'delivery_failed' }`.

### Bidirectional (2)
5. **Subordinate sends message to its supervisor** — delivered via supervisor session's input mechanism.
6. **Supervisor sends message to its subordinate** — delivered via subordinate's agent-to-agent channel.

### Any-to-Any (1)
7. **Direct supervisor-to-supervisor messaging** — two supervisors of same subordinate message each other directly via general-purpose channel (no ChainOfCommand link between them).

## Applicable Rules
20, 27, 31, 33

## Key Implementation Details

### New Infrastructure: Agent-to-Agent Channel
Every session gets a general-purpose `mpsc::channel(16)` separate from `supervisor_input`. This is the universal message delivery path:
- Supervisor→subordinate: route through agent-to-agent channel
- Subordinate→supervisor: route through agent-to-agent channel (or supervisor_input)
- Supervisor→supervisor: route through agent-to-agent channel (no ChainOfCommand link)

The `message` action abstracts the routing — callers specify `session_id` and `message`, the system determines the delivery path based on relationships.

### Message Parameters
- `session_id`: string (required) — target session
- `message`: string (required) — text content
- `context`: array (optional) — covered in AMGR-005

### Message Response
```json
{ "delivered": true, "session_id": "abc-123", "context_resolved": 0 }
```

### Delivery Failure
```json
{ "error": true, "code": "delivery_failed", "message": "Message channel full (16 pending messages)" }
```

### Queuing Behavior
- Target mid-generation → message waits in channel
- Next `tokio::select!` iteration picks it up
- No interruption of current generation
- Capacity 16 per channel

## Estimate: 5 points
Complex: new mpsc channel infrastructure, bidirectional routing logic, integration with existing supervisor_agent_loop select! loop.
