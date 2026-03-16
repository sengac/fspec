# AMGR-010 — Agent Messaging (Plain, Bidirectional, Any-to-Any)

## Summary

Implement the message action for AgentManager. Any session can send a plain text message to any other session by ID. This is the primary communication channel — supervisors send tasks, subordinates report results, peers coordinate.

## Action: message

### Parameters
- `session_id` (required) — target session to send to
- `message` (required) — text content
- `context` (optional) — covered in AMGR-011, not this story

### Behavior
- Looks up target session by ID
- Sends message through the target's agent-to-agent mpsc channel
- Message queues if target is mid-generation — picked up on next iteration
- No interruption of LLM generation in progress
- If channel is full (16 pending), fails with delivery_failed

### Response
- Success: `{ delivered: true, session_id: "target-id" }`
- Target not found: `{ error: true, code: "session_not_found", message: "..." }`
- Channel full: `{ error: true, code: "delivery_failed", message: "..." }`

## Infrastructure

### Agent-to-Agent Channel
Every BackgroundSession gets a new `mpsc::channel(16)` for incoming messages. This is separate from the prompt input channel. The agent_loop's tokio::select! picks up messages from this channel alongside prompt input.

### Message Flow
```
Sender session                    Target session
     │                                 │
     │ AgentManager(message, id, text) │
     │ ──────────────────────────────► │
     │    (handler looks up target,    │
     │     sends to target's mpsc)     │
     │                                 │ ◄── message appears in agent_loop
     │                                 │     as injected input on next turn
```

### Message Format (What the Receiver Sees)
The message is injected as a user-like turn. Format TBD but should clearly identify the sender:
```
<agent-message from="sender-session-id" role="security-reviewer">
Found 2 SQL injection vulnerabilities in query_builder.rs
</agent-message>
```

## Scenarios

1. Supervisor sends task to subordinate
2. Subordinate reports results back to supervisor
3. Subordinate interrupts supervisor with urgent finding
4. Message queues when target is processing
5. Delivery fails when channel is full
6. Message to non-existent session returns error
7. Peer-to-peer messaging between two subordinates of same supervisor
