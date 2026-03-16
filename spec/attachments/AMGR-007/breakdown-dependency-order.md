# AMGR-007 Breakdown — Dependency Order

## Dependency Chain

```
AMGR-008: Remove old supervisor infrastructure
    ├── AMGR-009: Core AgentManager tool (spawn, list, get_status, close)
    │       └── AMGR-010: Agent messaging (plain, bidirectional, any-to-any)
    │               └── AMGR-011: Message context resolution
    └── AMGR-012: Role management (set_role + /role TUI command)
```

AMGR-012 branches off AMGR-008 independently — roles don't need messaging or spawn to work.

## Implementation Order

### 1. AMGR-008 — Remove old supervisor infrastructure
**Depends on:** nothing
**Type:** Cleanup — no new features
**What:** Remove supervisor_agent_loop, ObservationBuffer, breakpoint detection, format_evaluation_prompt, automatic broadcast subscription, SupervisorInput injection, SupervisorRole struct, /supervisor TUI command, SupervisorTemplateList, SupervisorCreateView, SupervisorTemplateForm. Simplify ChainOfCommand to track spawner→spawned ownership only.
**Validates:** Codebase compiles, existing sessions work as regular agent_loop sessions, no regressions.

### 2. AMGR-009 — Core AgentManager tool (spawn, list, get_status, close)
**Depends on:** AMGR-008
**What:** New tool module at codelet/tools/src/agent_manager/ using handler-delegated pattern. Actions: spawn (creates subordinate with optional role, inherits model), list (all sessions with relationships), get_status (detailed info), close (terminate subordinate, spawner-only permission). Registered in all providers.
**Delivers:** Agents can create workers, discover sessions, check status, clean up.

### 3. AMGR-010 — Agent messaging (plain, bidirectional, any-to-any)
**Depends on:** AMGR-009
**What:** message action — send plain text to any session by ID. General-purpose mpsc channel (capacity 16) per session. Queuing behavior, delivery_failed on full. Works in all directions.
**Delivers:** Agents can communicate — supervisors send tasks, subordinates report results.

### 4. AMGR-011 — Message context resolution
**Depends on:** AMGR-010
**What:** Optional context array on message — specific turns, turn ranges, search queries. Resolved at send time. XML-style format. Graceful degradation.
**Delivers:** Rich messaging with quoted conversation history.

### 5. AMGR-012 — Role management (set_role + /role TUI)
**Depends on:** AMGR-008 (parallel with AMGR-009+)
**What:** set_role action sets a role string (system prompt overlay) on any session. /role TUI command opens dialog for current session. Replaces /supervisor command.
**Delivers:** Any session can have behavioral instructions applied.
