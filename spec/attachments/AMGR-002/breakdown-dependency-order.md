# AMGR-002 Breakdown — Dependency Order

## Overview

AMGR-002 (AgentManager Tool) is broken into 4 child stories forming a linear dependency chain. Each builds on the previous, delivering incremental value.

## Dependency Chain

```
WATCH-024 (DONE ✅)
    └── AMGR-003: Core Tool Infrastructure + Agent Lifecycle
            └── AMGR-004: Agent Messaging — Plain, Bidirectional, Any-to-Any
                    └── AMGR-005: Message Context Resolution
                            └── AMGR-006: Inter-Agent Coordination Patterns
```

## Implementation Order

### 1. AMGR-003 — Core Tool Infrastructure + Agent Lifecycle
**Depends on:** WATCH-024 (done)
**Scenarios:** 12 (spawn ×2, list ×2, get_status ×1, set_role ×1, close ×2, error handling ×1, tool registration ×2, mutual awareness ×1)
**Delivers:** The minimum viable AgentManager — agents can create, discover, inspect, modify, and terminate supervisor sessions programmatically. Replaces the TUI-only supervisor creation path.
**Key infrastructure:** Tool module at `codelet/tools/src/agent_manager.rs`, NAPI handler, action dispatch, JSON error format, reuses existing supervisor infrastructure.

### 2. AMGR-004 — Agent Messaging — Plain, Bidirectional, Any-to-Any
**Depends on:** AMGR-003
**Scenarios:** 7 (plain message ×2, queuing ×1, delivery failure ×1, subordinate→supervisor ×1, supervisor→subordinate ×1, supervisor→supervisor ×1)
**Delivers:** Directed communication between any two sessions. Agents can send plain text messages regardless of ChainOfCommand relationship.
**Key infrastructure:** General-purpose agent-to-agent mpsc channel (separate from supervisor_input), message routing abstraction.

### 3. AMGR-005 — Message Context Resolution
**Depends on:** AMGR-004
**Scenarios:** 4 (specific turns ×1, cross-session multi-source ×1, zero-match query ×1, non-existent session ×1)
**Delivers:** Rich messaging with quoted conversation history. Agents can ground messages in specific turns, ranges, or search results from any session. Graceful degradation for stale references.
**Key infrastructure:** Context resolution engine using SessionSearch's persistence layer, resolved context formatting.

### 4. AMGR-006 — Inter-Agent Coordination Patterns
**Depends on:** AMGR-005
**Scenarios:** 4 (full coordination loop ×1, supervisor inspect+message ×1, peer supervisor coordination ×1, bidirectional discussion ×1)
**Delivers:** Validated end-to-end coordination proving the discover→inspect→communicate→verify loop works. No new infrastructure — integration scenarios only.

## Scenario Distribution

| Story | Scenarios | New Infrastructure | Estimate |
|-------|-----------|-------------------|----------|
| AMGR-003 | 12 | Tool module, NAPI, dispatch, lifecycle actions | 8 |
| AMGR-004 | 7 | Agent-to-agent mpsc, routing | 5 |
| AMGR-005 | 4 | Context resolution, formatting | 5 |
| AMGR-006 | 4 | None (integration only) | 3 |
| **Total** | **27** | | **21** |

## Rule Distribution

### AMGR-003 (Lifecycle)
- Rule 0: Single tool with action dispatch
- Rule 7: SessionSearch/DeepSearch remain separate
- Rule 9: Tool module location and registration
- Rule 11 (updated): spawn accepts role, brief, auto_inject — model inherited
- Rule 12: set_role changes role/brief/auto_inject
- Rule 13: spawn reuses existing supervisor infrastructure
- Rule 15: close cleans up ChainOfCommand and broadcast
- Rule 16: Replaces TUI-only supervisor creation
- Rule 17: get_status returns detailed session info
- Rule 18: list returns all sessions
- Rule 22: Session IDs as first-class coordination handles
- Rule 23: Mutual awareness on spawn
- Rule 24: No access control boundaries (ambient trust)
- Rule 28: Close is the one privileged action
- Rule 32: Consistent JSON error format
- Rule 34: No maximum supervisor limit

### AMGR-004 (Messaging)
- Rule 20: Any-to-any directed messaging
- Rule 27: Plain message with no context is valid
- Rule 31: True any-to-any via general-purpose mpsc channel
- Rule 33: Messages queue (capacity 16), delivery_failed on full

### AMGR-005 (Context Resolution)
- Rule 25: Context array with turn numbers, ranges, or queries
- Rule 26: Context mirrors sender's SessionSearch workflow
- Rule 29: Zero-match query → warning note, message still delivers
- Rule 30: Non-existent session → warning note, message still delivers
- Rule 35: Three context variants (turns, range, query)

### AMGR-006 (Coordination)
- Rule 19: Discover→inspect→communicate→verify loop
- Rule 21: SessionSearch as cross-agent inspection layer

## Success Response Shapes (All Stories)

| Action | Response | Story |
|--------|----------|-------|
| spawn | `{ session_id, role, brief }` | AMGR-003 |
| list | `{ sessions: [{ session_id, name, role, status, subordinate_id, supervisor_ids, supervisor_count }] }` | AMGR-003 |
| get_status | `{ session_id, role, brief, auto_inject, subordinate_id, supervisor_ids, status, model, pending_messages }` | AMGR-003 |
| set_role | `{ session_id, role, brief, auto_inject, previous_role }` | AMGR-003 |
| close | `{ closed: true, session_id, role }` | AMGR-003 |
| message | `{ delivered: true, session_id, context_resolved: number }` | AMGR-004/005 |

Error (all actions): `{ error: true, code: string, message: string }`
