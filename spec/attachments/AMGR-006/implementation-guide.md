# AMGR-006 — Inter-Agent Coordination Patterns

## Summary

Integration-level scenarios that validate the full discover→inspect→communicate→verify coordination loop emerging from composing AgentManager with SessionSearch. No new infrastructure — these scenarios prove the system works end-to-end across tool boundaries.

## Depends On
- **AMGR-005** — message context resolution (needed for inspect+message scenarios)

## Scenarios (4)

### Full Loop (1)
1. **Full discover-inspect-communicate-verify coordination loop** — spawn supervisor → list to discover session ID → SessionSearch(search) to check findings → SessionSearch(show) to read context → message with follow-up question. Each step uses session IDs as handles across both tools.

### Supervisor Inspects Subordinate (1)
2. **Supervisor inspects subordinate history via SessionSearch then messages with context** — supervisor searches for topic across sessions → discovers subordinate discussed schema change → sends message with context referencing those turns → subordinate receives commentary with referenced history inlined.

### Peer Coordination (1)
3. **Peer supervisors coordinate by reading each other's findings** — subordinate spawns security-reviewer and test-writer → test-writer calls SessionSearch(show) on security-reviewer's session → discovers auth bypass concern → writes tests covering that attack vector → all without subordinate coordinating between them.

### Bidirectional Discussion (1)
4. **Bidirectional discussion between subordinate and supervisor** — subordinate receives message → calls SessionSearch(show) on supervisor to understand reasoning → sends counter-argument via message with context referencing own conversation → supervisor receives response with context → discussion continues through further exchanges.

## Applicable Rules
19, 21

## Key Validation Points

### What These Scenarios Prove
- Session IDs flow seamlessly between AgentManager and SessionSearch
- AgentManager's `list` output is directly usable by SessionSearch's `session_id` parameter
- Agents can discover each other, inspect history, communicate, and verify outcomes
- Peer supervisors coordinate independently without subordinate mediation
- Bidirectional multi-turn discussions work naturally
- The two tools compose by convention, not coupling

### What These Scenarios Do NOT Test
- Individual action behavior (covered by AMGR-003/004/005)
- Error handling (covered by AMGR-003)
- Edge cases like full channels or missing sessions (covered by AMGR-004/005)

### Test Strategy
These are integration tests that exercise the full tool stack. They may require:
- Multiple concurrent agent sessions
- Actual SessionSearch calls against persisted conversation history
- Verifying that resolved context in messages matches what SessionSearch would return directly

## Estimate: 3 points
Moderate: integration scenarios only, no new infrastructure, but requires multi-session test setup.
