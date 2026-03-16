# AMGR-007 — AgentManager Tool (Revised Model)

## Core Concept

The spawner is the **supervisor**. Spawned sessions are **subordinates** (workers). This is the natural model: "I need help, I create a worker, I tell it what to do, I collect results, I close it when done."

## Model Inversion from AMGR-002

The old AMGR-002 model was **inverted** — agents spawned "supervisors" above themselves that observed their stream and injected advice. The new model flips this:

| Old (AMGR-002) | New (AMGR-007) |
|---|---|
| Spawner = subordinate | Spawner = supervisor |
| Spawned = supervisor (observes parent) | Spawned = subordinate (does work for parent) |
| Context pushed via broadcast observation | Context pulled via SessionSearch |
| Advice injected automatically via SupervisorInput | Results sent explicitly via message |
| supervisor_agent_loop with tokio::select! | Regular agent_loop for all sessions |
| ObservationBuffer, breakpoint detection | Removed — not needed |
| /supervisor TUI command | Removed — replaced by /role + AgentManager |

## Actions

| Action | Purpose |
|---|---|
| **spawn** | Create a subordinate session, optionally with a role. Returns session_id. |
| **message** | Send a message to any session by ID. Any-to-any. |
| **list** | List all sessions with IDs, roles, status, relationships. |
| **get_status** | Detailed info for a specific session. |
| **set_role** | Set/change role (system prompt overlay) on any session. |
| **close** | Terminate a subordinate. Only spawner or user can close. |

## Workflow

```
1. Supervisor calls spawn(role="security reviewer") → gets subordinate_id
2. Supervisor calls message(session_id=subordinate_id, message="Review src/auth/ for vulns")
3. Subordinate works... uses SessionSearch to check supervisor's context if needed
4. Subordinate calls message(session_id=supervisor_id, message="Found 2 issues...")
5. Supervisor reads result, decides it's done
6. Supervisor calls close(session_id=subordinate_id)
```

## Role

A role is a simple string — like a system prompt overlay. Any session can have one:
- Set via AgentManager(action='set_role', session_id=X, role='...')
- Set via /role TUI command (opens dialog for current session)
- Optionally set at spawn time

## What Gets Removed

- `supervisor_agent_loop` — replaced by regular `agent_loop`
- `ObservationBuffer` — no automatic observation
- Breakpoint detection — not needed
- `format_evaluation_prompt` — not needed
- Automatic broadcast subscription for observation — not needed
- `SupervisorInput` injection pipeline — replaced by message action
- `SupervisorRole` struct — replaced by simple role string
- `/supervisor` TUI command — removed
- `SupervisorTemplateList`, `SupervisorCreateView`, `SupervisorTemplateForm` — removed

## What Stays (Modified)

- `ChainOfCommand` — stays but tracks spawner→spawned ownership (not observation)
- `BackgroundSession` — stays, all sessions are the same
- `SessionManager` — stays, manages all sessions
- `broadcast::channel` — may still be useful for TUI display, but not for agent observation
