# AMGR-011 — Message Context Resolution

## Summary

Extend the message action to support an optional context array of session history references. Agents can quote specific conversation history when messaging, so the receiver gets a self-contained message without needing to do follow-up SessionSearch calls.

## Context Reference Variants

### 1. Specific turns
```json
{ "session_id": "abc-123", "turns": [42, 43, 44] }
```

### 2. Turn range
```json
{ "session_id": "abc-123", "start_turn": 10, "end_turn": 15 }
```

### 3. Search query
```json
{ "session_id": "abc-123", "query": "SQL injection" }
```

All three can be mixed in a single context array.

## Resolution

- Happens at send time in the AgentManager message handler
- Uses the same persistence layer as SessionSearch (MessageStore, SessionStore)
- Resolved content appended after the sender's message text

## Resolved Format

```xml
<agent-message from="sender-id" role="security-reviewer">
This pattern is vulnerable to the same injection we discussed earlier

<quoted-context>
<from session="abc-123" turns="42-44">
[42] user: How should we handle the admin endpoint?
[43] assistant: The admin endpoint should validate JWT tokens...
[44] user: What about SQL injection?
</from>
</quoted-context>
</agent-message>
```

## Graceful Degradation

- Query matches nothing: `<from session="X" query="term">⚠ No matches for query "term"</from>`
- Session not found: `<from session="deleted-id" turns="1-2">⚠ Session deleted-id not found</from>`
- Message still delivers with sender's text intact
- `context_resolved` count in response indicates how many references resolved successfully

## Response

```json
{ "delivered": true, "session_id": "target-id", "context_resolved": 2 }
```

## Scenarios

1. Message with specific turn references — content inlined
2. Message with cross-session references from multiple sources
3. Message with search query reference
4. Message with turn range reference
5. Context reference to non-existent session — warning, message delivers
6. Context reference with zero-match query — warning, message delivers
7. Mixed context array (turns + query from different sessions)
