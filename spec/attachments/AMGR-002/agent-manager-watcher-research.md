# AgentManager Tool — Research: Watcher System Architecture & Replacement Design

## Purpose

This document captures the deep research conducted on the existing watcher system to inform the design of the AgentManager tool (AMGR-002). The AgentManager tool will expose agent orchestration, role management, and cross-agent communication as a tool call, replacing the TUI-only watcher creation workflow.

---

## 1. Existing Watcher System Architecture

### 1.1 WatchGraph (WATCH-002)

The `WatchGraph` is a relationship tracker owned by `SessionManager` in `codelet/napi/src/session_manager.rs`:

```rust
pub struct WatchGraph {
    parent_to_watchers: RwLock<HashMap<Uuid, Vec<Uuid>>>,  // 1:N
    watcher_to_parent: RwLock<HashMap<Uuid, Uuid>>,        // 1:1
}
```

**Key constraints:**
- One watcher → one parent (1:1 from watcher side)
- One parent → many watchers (1:N from parent side)
- Circular watching prevented via ancestor chain traversal during `add_watcher()`
- Ephemeral — relationships do not persist across restarts

**Core methods:** `add_watcher()`, `remove_watcher()`, `get_watchers()`, `get_parent()`, `remove_session_relationships()`

### 1.2 Broadcast Channel (WATCH-003)

Every `BackgroundSession` has a `tokio::sync::broadcast` channel (capacity 256):

```rust
watcher_broadcast: broadcast::Sender<StreamChunk>,
```

- In `handle_output()`, every chunk emitted by the session is also sent to the broadcast
- Fire-and-forget — if no receivers are subscribed, the send is silently ignored
- Watcher sessions call `parent.subscribe_to_stream()` → gets a `broadcast::Receiver<StreamChunk>`
- Late subscribers start from current position (no replay)
- Slow receivers get `RecvError::Lagged(n)` if they fall >256 chunks behind

### 1.3 Session Roles & Authority Model (WATCH-004)

```rust
pub enum RoleAuthority {
    Peer,        // Suggestions that parent may consider
    Supervisor,  // Directives that should be followed
}

pub struct SessionRole {
    pub name: String,
    pub description: Option<String>,
    pub authority: RoleAuthority,
    pub auto_inject: bool,  // WATCH-020: autonomous injection toggle
}
```

Authority affects:
1. The evaluation prompt — Supervisors told "your interjections should be followed"
2. The formatted injection prefix — `[WATCHER: role | Authority: Peer/Supervisor | Session: id]`

### 1.4 Watcher Agent Loop (WATCH-005, WATCH-019)

The watcher runs a specialized `watcher_agent_loop` (NOT the regular `agent_loop`):

#### WatcherState
```rust
pub enum WatcherState { Idle, Observing, Processing }
```

#### ObservationBuffer
Accumulates parent's `StreamChunk`s with methods: `push()`, `accumulated_text()`, `correlation_ids()`

#### Dual Input via `watcher_loop_tick()`
Uses `tokio::select! { biased; ... }` to multiplex THREE input sources:
1. **User input channel** (HIGHEST PRIORITY)
2. **Parent broadcast receiver** (observations)
3. **Silence timeout** (fires if buffer non-empty after 5s)

Returns `WatcherLoopAction`: `ProcessUserPrompt`, `ProcessObservations`, `Continue`, `Stop`

#### Natural Breakpoints
Evaluation triggered when buffer has content AND:
- `StreamChunk::Done` — parent turn completed
- `StreamChunk::ToolResult` — tool execution finished
- Silence timeout — 5 seconds with no new chunks

### 1.5 Interjection Parsing (WATCH-020)

#### Evaluation Prompt
```
You are a watcher session with role: {name}
Authority level: {level} - {context}
=== PARENT SESSION OBSERVATIONS ===
{accumulated text, tool calls, tool results}
=== END OBSERVATIONS ===

RESPONSE FORMAT (required):
If inject: [INTERJECT] urgent: true/false content: ... [/INTERJECT]
If no: [CONTINUE] reasoning [/CONTINUE]
```

#### Auto-Inject Flow
1. Observation evaluation completes → `get_turn_text()`
2. `parse_interjection(turn_text)` → `Some(Interjection { urgent, content })` or `None`
3. If `auto_inject=true` → calls `watcher_inject()` directly
4. If `auto_inject=false` → emits `StreamChunk::WatcherPendingInjection` for UI review

### 1.6 Injection Messages (WATCH-006)

```rust
pub struct WatcherInput {
    pub source_session_id: String,
    pub role_name: String,
    pub authority: RoleAuthority,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}
```

Each `BackgroundSession` has an mpsc channel (capacity 16). Parent's `agent_loop` reads from `watcher_input_rx` via `tokio::select!`, formats with structured prefix, and processes as LLM input.

### 1.7 Cross-Pane Correlation IDs (WATCH-011)

Every `StreamChunk` has:
- `correlation_id: Option<String>` — unique ID assigned by parent
- `observed_correlation_ids: Option<Vec<String>>` — parent IDs that triggered watcher response

Enables bidirectional cross-pane highlighting in the TUI split view.

---

## 2. Existing NAPI Bindings

| Function | Purpose |
|---|---|
| `sessionCreateWatcher(parentId, model, project, name)` | Create watcher session |
| `sessionSetRole(sessionId, roleName, desc, authority, autoInject)` | Set role |
| `sessionGetRole(sessionId)` | Get role info |
| `sessionClearRole(sessionId)` | Remove role |
| `sessionGetParent(sessionId)` | Get parent session ID |
| `sessionGetWatchers(sessionId)` | Get all watcher IDs |
| `watcherInject(watcherId, message)` | Inject message to parent |
| `sessionSetObservedCorrelationIds(sessionId, ids)` | Tag output chunks |
| `sessionClearObservedCorrelationIds(sessionId)` | Stop tagging |

---

## 3. TUI Watcher System

### 3.1 Watcher Creation Flow (3 entry points)
1. **WatcherCreateView.tsx** — form-based creation from `/watcher` overlay
2. **Template Spawn** — one-click from template list
3. **Slash Command** — `/watcher spawn <slug>`

All paths call `sessionCreateWatcher()` → `sessionSetRole()`.

### 3.2 Watcher Templates
- Stored in `~/.fspec/watcher-templates.json`
- No built-in presets — entirely user-driven
- Template fields: `id, name, slug, modelId, authority, brief, autoInject, createdAt, updatedAt`
- CRUD via `watcherTemplateStorage.ts` utilities

### 3.3 Split View (WATCH-010)
- Left pane: Parent session (read-only, dimmed)
- Right pane: Watcher conversation (interactive)
- Single input area sends to watcher
- Cross-pane highlighting via correlation IDs

---

## 4. AgentManager Tool Design

### 4.1 Why AgentManager?

Currently, agents CANNOT create watchers — only the user can via the TUI overlay. The AgentManager tool exposes agent lifecycle management as a tool call, enabling:
- Agents to spawn specialized sub-agents autonomously
- Agents to reassign roles mid-session
- Agents to send directed messages to other agents
- Agents to query the status of their agent ecosystem

### 4.2 Actions

| Action | Parameters | Returns |
|---|---|---|
| `spawn` | `role`, `brief`, `authority?`, `model?`, `auto_inject?` | `{ session_id, role, authority }` |
| `set_role` | `session_id`, `role`, `brief?`, `authority?`, `auto_inject?` | `{ session_id, role, authority, auto_inject }` |
| `list` | (none) | `[{ session_id, name, role, authority, status, parent_id, watcher_count }]` |
| `get_status` | `session_id` | `{ session_id, role, authority, auto_inject, parent_id, watcher_ids, state, model, message_count }` |
| `message` | `session_id`, `message` | `{ delivered: true }` |
| `close` | `session_id` | `{ closed: true }` |

### 4.3 Implementation Strategy

1. **Tool module:** `codelet/tools/src/agent_manager.rs`
2. **NAPI handler:** Registered in session_manager.rs alongside SessionSearch
3. **Delegates to existing NAPI functions:** `sessionCreateWatcher`, `sessionSetRole`, `watcherInject`, etc.
4. **No new watcher infrastructure needed** — purely an API surface on existing capabilities
5. **TUI watcher overlay continues to work** — both paths call the same underlying functions

### 4.4 Relationship to Other Tools

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ SessionSearch │    │ AgentManager │    │  DeepSearch   │
│  (AMGR-001)  │    │  (AMGR-002)  │    │  (RLM-001)   │
│              │    │              │    │              │
│ • recent     │    │ • spawn      │    │ • query      │
│ • search     │    │ • set_role   │    │ • scope      │
│ • show       │    │ • list       │    │              │
│              │    │ • get_status │    │ Ephemeral    │
│ History      │    │ • message    │    │ sub-agent    │
│ access       │    │ • close      │    │              │
└──────────────┘    └──────────────┘    └──────────────┘
       │                   │                    │
       └───────────┬───────┘                    │
                   │                            │
              Separate tools               Separate tool
              Same persistence             Own agent loop
```

### 4.5 set_role — The Key Differentiator

The `set_role` action is what makes AgentManager more than just "spawn and forget":
- Change any session's role, description, authority, and auto_inject settings at runtime
- Enable dynamic reconfiguration: start a generic agent, then assign it a specific role
- Elevate/demote authority without restarting the session
- Toggle auto_inject on/off based on context

This replaces the static watcher template approach with dynamic role assignment.

---

## 5. Migration Path from Watchers to AgentManager

The watcher system is **not being removed** — AgentManager wraps it. The TUI watcher overlay (`/watcher`) continues to work alongside the tool call. Over time, as agents learn to use AgentManager, the TUI-only path becomes less central. The key shift is:

| Before (Watcher-only) | After (AgentManager) |
|---|---|
| Only user can create watchers (TUI) | Agent can spawn sub-agents (tool call) |
| Role set at creation, rarely changed | Role can be set/changed anytime |
| Templates stored in user config | Agent decides role parameters dynamically |
| Manual watcher management | Agent manages its own sub-agent ecosystem |

---

## 6. Key Design Decisions

1. **Single tool, action dispatch** — Same pattern as SessionSearch, Bridge
2. **Reuse existing infrastructure** — No parallel watcher implementation
3. **spawn defaults to caller's model** — Agent doesn't need to know model IDs
4. **authority defaults to peer** — Safe default, can be elevated via set_role
5. **auto_inject defaults to true** — Matches current watcher behavior
6. **message uses existing injection channel** — WatcherInput mpsc, formatted with role prefix
7. **close cascades cleanup** — WatchGraph removal, broadcast unsubscribe
