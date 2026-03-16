# AMGR-005 — Message Context Resolution

## Summary

Extend the `message` action to support an optional `context` array of session history references. The system resolves references at send time — fetching actual conversation content from the persistence layer — and delivers a self-contained message with history inlined. Graceful degradation for missing sessions and zero-match queries.

## Depends On
- **AMGR-004** — plain messaging infrastructure

## Scenarios (4)

### Specific Turns (1)
1. **Send message with specific turn references** — `context: [{session_id: 'X', turns: [42, 43, 44]}]` → message delivered with content of turns 42-44 inlined. Receiver gets self-contained message.

### Cross-Session Multi-Source (1)
2. **Send message with cross-session context from multiple sources** — `context: [{session_id: 'A', query: 'SQL injection'}, {session_id: 'B', turns: [8, 9]}]` → both referenced contexts resolved and inlined in a single delivery.

### Graceful Degradation (2)
3. **Context reference with query matching zero turns** — query resolves to nothing → warning note `--- No matches for query "nonexistent term" in session X ---` included, message still delivers with sender's text intact.
4. **Context reference to non-existent session** — session gone → warning note `--- Session abc-123 not found ---` included, message still delivers.

## Applicable Rules
25, 26, 29, 30, 35

## Key Implementation Details

### Three Context Reference Variants

```typescript
// Specific turns
{ session_id: "abc-123", turns: [42, 43, 44] }

// Turn range
{ session_id: "abc-123", start_turn: 10, end_turn: 15 }

// Search query
{ session_id: "abc-123", query: "SQL injection" }
```

All three can be mixed in a single context array.

### Resolution Engine
- Reuses the same persistence layer as SessionSearch (MessageStore, SessionStore)
- Resolution happens in the AgentManager message handler BEFORE formatting the delivery payload
- Resolved content appended as a structured block after the sender's message text
- Clearly delineated so the receiving LLM distinguishes sender's commentary from referenced history

### Resolved Context Format (Proposed)
```
--- Context from session <id> (turns 42-44) ---
[turn 42] user: How should we handle the admin endpoint?
[turn 43] assistant: The admin endpoint should validate JWT tokens...
[turn 44] user: What about SQL injection?
--- End context ---
```

### Graceful Degradation Principle
The message text is the PRIMARY payload. Context is supplementary. Never fail the entire message because one reference is stale or a query returns nothing. Include informational warnings in the resolved block and deliver everything that did resolve.

### Message Response (with context)
```json
{ "delivered": true, "session_id": "abc-123", "context_resolved": 2 }
```
Where `context_resolved` = number of references that successfully resolved content (excluding warnings).

## Estimate: 5 points
Complex: context resolution engine, three variant types, persistence layer integration, graceful degradation logic, formatting.
