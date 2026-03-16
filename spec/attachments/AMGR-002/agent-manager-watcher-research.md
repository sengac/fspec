# AgentManager Tool — Research: Supervisor System Architecture & Inter-Agent Coordination Design

## Purpose

This document captures the deep research conducted on the existing supervisor system and the design of the AgentManager tool (AMGR-002). AgentManager exposes agent orchestration, role management, and cross-agent communication as a tool call. Combined with SessionSearch for cross-session history inspection, it enables a complete inter-agent coordination loop: discover → inspect → communicate → verify.

---

## 1. Existing Supervisor System Architecture

### 1.1 ChainOfCommand (WATCH-002, renamed by WATCH-024)

The `ChainOfCommand` is a relationship tracker owned by `SessionManager` in `codelet/napi/src/session_manager.rs`:

```rust
pub struct ChainOfCommand {
    subordinate_to_supervisors: RwLock<HashMap<Uuid, Vec<Uuid>>>,  // 1:N
    supervisor_to_subordinate: RwLock<HashMap<Uuid, Uuid>>,        // 1:1
}
```

**Key constraints:**
- One supervisor → one subordinate (1:1 from supervisor side)
- One subordinate → many supervisors (1:N from subordinate side)
- Circular watching prevented via ancestor chain traversal during `add_supervisor()`
- Ephemeral — relationships do not persist across restarts

**Core methods:** `add_supervisor()`, `remove_supervisor()`, `get_supervisors()`, `get_subordinate()`, `remove_session_relationships()`

### 1.2 Broadcast Channel (WATCH-003)

Every `BackgroundSession` has a `tokio::sync::broadcast` channel (capacity 256):

```rust
supervisor_broadcast: broadcast::Sender<StreamChunk>,
```

- In `handle_output()`, every chunk emitted by the session is also sent to the broadcast
- Fire-and-forget — if no receivers are subscribed, the send is silently ignored
- Supervisor sessions call `subordinate.subscribe_to_stream()` → gets a `broadcast::Receiver<StreamChunk>`
- Late subscribers start from current position (no replay)
- Slow receivers get `RecvError::Lagged(n)` if they fall >256 chunks behind

### 1.3 SupervisorRole (WATCH-004, simplified by WATCH-024)

```rust
pub struct SupervisorRole {
    pub name: String,
    pub brief: Option<String>,
    pub auto_inject: bool,  // WATCH-020: autonomous injection toggle
}
```

The `RoleAuthority` enum (Peer/Supervisor) was **removed entirely** by WATCH-024. The `brief` field (renamed from `description`) provides all behavioral instruction — no artificial authority levels needed.

### 1.4 Supervisor Agent Loop (WATCH-005, WATCH-019, renamed by WATCH-024)

The supervisor runs a specialized `supervisor_agent_loop` (NOT the regular `agent_loop`):

#### SupervisorState
```rust
pub enum SupervisorState { Idle, Observing, Processing }
```

#### ObservationBuffer
Accumulates subordinate's `StreamChunk`s with methods: `push()`, `accumulated_text()`, `correlation_ids()`

#### Dual Input via `supervisor_loop_tick()`
Uses `tokio::select! { biased; ... }` to multiplex THREE input sources:
1. **User input channel** (HIGHEST PRIORITY)
2. **Subordinate broadcast receiver** (observations)
3. **Silence timeout** (fires if buffer non-empty after 5s)

Returns `SupervisorLoopAction`: `ProcessUserPrompt`, `ProcessObservations`, `Continue`, `Stop`

#### Natural Breakpoints
Evaluation triggered when buffer has content AND:
- `StreamChunk::Done` — subordinate turn completed
- `StreamChunk::ToolResult` — tool execution finished
- Silence timeout — 5 seconds with no new chunks

### 1.5 Interjection Parsing (WATCH-020)

#### Evaluation Prompt
```
You are a supervisor session with role: {name}
{brief}
=== SUBORDINATE SESSION OBSERVATIONS ===
{accumulated text, tool calls, tool results}
=== END OBSERVATIONS ===

RESPONSE FORMAT (required):
If inject: [INTERJECT] urgent: true/false content: ... [/INTERJECT]
If no: [CONTINUE] reasoning [/CONTINUE]
```

#### Auto-Inject Flow
1. Observation evaluation completes → `get_turn_text()`
2. `parse_interjection(turn_text)` → `Some(Interjection { urgent, content })` or `None`
3. If `auto_inject=true` → calls `supervisor_inject()` directly (internal Rust only, no NAPI export)
4. If `auto_inject=false` → emits `StreamChunk::SupervisorPendingInjection` for UI review

### 1.6 Injection Messages (WATCH-006, renamed by WATCH-024)

```rust
pub struct SupervisorInput {
    pub source_session_id: String,
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}
```

Each `BackgroundSession` has an mpsc channel (capacity 16). Subordinate's `agent_loop` reads from `supervisor_input_rx` via `tokio::select!`, formats with structured prefix `[SUPERVISOR: role | Session: id]`, and processes as LLM input.

### 1.7 Cross-Pane Correlation IDs (WATCH-011)

Every `StreamChunk` has:
- `correlation_id: Option<String>` — unique ID assigned by subordinate
- `observed_correlation_ids: Option<Vec<String>>` — subordinate IDs that triggered supervisor response

Enables bidirectional cross-pane highlighting in the TUI split view.

---

## 2. Existing NAPI Bindings (post WATCH-024)

| Function | Purpose |
|---|---|
| `sessionCreateSupervisor(subordinateId, model, project, name)` | Create supervisor session |
| `sessionSetRole(sessionId, roleName, brief, autoInject)` | Set role |
| `sessionGetRole(sessionId)` | Get role info |
| `sessionGetSubordinate(sessionId)` | Get subordinate session ID |
| `sessionGetSupervisors(sessionId)` | Get all supervisor IDs |
| `sessionSetObservedCorrelationIds(sessionId, ids)` | Tag output chunks |
| `sessionClearObservedCorrelationIds(sessionId)` | Stop tagging |

**Removed by WATCH-024:**
- `sessionClearRole` — dead code with no consumers
- `watcherInject` (NAPI export) — `supervisor_inject` is now internal Rust only, called by auto-inject

---

## 3. TUI Supervisor System

### 3.1 Supervisor Creation Flow (3 entry points)
1. **SupervisorCreateView.tsx** — form-based creation from `/supervisor` overlay
2. **Template Spawn** — one-click from template list
3. **Slash Command** — `/supervisor spawn <slug>`

All paths call `sessionCreateSupervisor()` → `sessionSetRole()`.

### 3.2 Supervisor Templates
- Stored in `~/.fspec/supervisor-templates.json`
- No built-in presets — entirely user-driven
- Template fields: `id, name, slug, modelId, brief, autoInject, createdAt, updatedAt`
- CRUD via `supervisorTemplateStorage.ts` utilities

### 3.3 Split View (WATCH-010)
- Left pane: Subordinate session (read-only, dimmed)
- Right pane: Supervisor conversation (interactive)
- Single input area sends to supervisor
- Cross-pane highlighting via correlation IDs

---

## 4. AgentManager Tool Design

### 4.1 Why AgentManager?

Currently, agents CANNOT create supervisors — only the user can via the TUI overlay. The AgentManager tool exposes agent lifecycle management as a tool call, enabling:
- Agents to spawn specialized supervisor agents autonomously
- Agents to reassign roles mid-session
- Agents to send directed messages to other agents — with optional context references
- Agents to query the status of their agent ecosystem
- Agents to coordinate with each other by composing AgentManager with SessionSearch

### 4.2 Actions

| Action | Parameters | Returns |
|---|---|---|
| `spawn` | `role`, `brief`, `model?`, `auto_inject?` | `{ session_id, role }` |
| `set_role` | `session_id`, `role`, `brief?`, `auto_inject?` | `{ session_id, role, auto_inject }` |
| `list` | (none) | `[{ session_id, name, role, status, subordinate_id, supervisor_count }]` |
| `get_status` | `session_id` | `{ session_id, role, brief, auto_inject, subordinate_id, supervisor_ids, state, model, message_count }` |
| `message` | `session_id`, `message`, `context?` | `{ delivered: true }` |
| `close` | `session_id` | `{ closed: true }` |

### 4.3 The `message` Action — Context References

The `message` action accepts not just a plain string but also an optional `context` array of session history references. Each reference specifies:

- `session_id` — which session's history to reference
- `turns` — specific turn numbers (e.g., `[42, 43, 44]`)
- `start_turn` / `end_turn` — a range of turns
- `query` — a search term to find matching turns in the target session

**Resolution happens at send time.** The system resolves references using the same persistence layer as SessionSearch (MessageStore, SessionStore), inlines the actual conversation content, and delivers a self-contained message to the receiver.

The receiver gets:
```
[SUPERVISOR: security-reviewer | Session: abc-123]
This pattern is vulnerable to the same injection we discussed earlier

=== Referenced Context ===
--- Session: def-456, Turns 42-44 ---
[Turn 42] User: Can you check the query builder for SQL injection?
[Turn 43] Assistant: I found two injection points in buildWhereClause()...
[Turn 44] User: Fix those and add parameterized queries
=== End Referenced Context ===
```

**Why context references matter:**
- The sending agent has already done the discovery work — it searched, found relevant turns, understands the context
- Forcing the receiving agent to redo that search is wasteful
- This is how humans communicate: you quote the relevant part and add your commentary, you don't just say "go read email #47"
- A plain message with no `context` array is still valid — context is optional for simple directives

**Cross-session context:** A single message can reference turns from multiple sessions, enabling an agent to cross-reference findings from several peers in one communication:

```
context: [
  { session_id: "<security-reviewer-id>", query: "SQL injection" },
  { session_id: "<test-writer-id>", turns: [8, 9] }
]
```

### 4.4 Implementation Strategy

1. **Tool module:** `codelet/tools/src/agent_manager.rs`
2. **NAPI handler:** Registered in session_manager.rs alongside SessionSearch
3. **Delegates to existing NAPI functions:** `sessionCreateSupervisor`, `sessionSetRole`, etc.
4. **No new supervisor infrastructure needed** — purely an API surface on existing capabilities
5. **TUI supervisor overlay continues to work** — both paths call the same underlying functions
6. **Message context resolution** reuses SessionSearch's persistence layer — resolves references before formatting the SupervisorInput payload, appending resolved content as a structured block after the sender's message text

---

## 5. Inter-Agent Coordination Architecture

### 5.1 The Core Insight: No Conversation Forking

Each agent has its own **linear conversation**, its own context window, its own session. There is no need for conversation forking, branching, merging, or any of that complexity.

An agent that needs context from another agent just reads it via SessionSearch. The other agent's conversation stays untouched. When they need to talk, `message` drops a message into the other agent's stream. That agent processes it in its own turn, in its own context.

The whole multi-agent coordination problem collapses into:
- **N independent linear conversations** (one per agent)
- **Read access across all of them** (SessionSearch)
- **Point-to-point messaging with context** (AgentManager message action)

No fork/merge semantics, no conversation trees, no conflict resolution. Each agent is sovereign over its own conversation history.

### 5.2 Two Tools, One Coordination Pattern

| Tool | Responsibility |
|------|---------------|
| **AgentManager** | Identity + Communication — who exists, spawn/close, send messages with context |
| **SessionSearch** | Inspection — read any agent's full conversation history, search across all sessions |

Neither tool imports or depends on the other. They compose by convention — session IDs are the coordination handles passed between them.

### 5.3 The Coordination Loop

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  DISCOVER   │────▶│   INSPECT   │────▶│ COMMUNICATE │────▶│   VERIFY    │
│             │     │             │     │             │     │             │
│ AgentManager│     │SessionSearch│     │ AgentManager │     │SessionSearch│
│ list        │     │ show/search │     │ message     │     │ show/search │
│ get_status  │     │ (other IDs) │     │ (+context)  │     │ (other IDs) │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
       │                                                            │
       └────────────────────────────────────────────────────────────┘
                              (repeat)
```

1. **Discover** — Agent calls `list` to find all sessions, gets their IDs, roles, status
2. **Inspect** — Agent calls `SessionSearch(action='show', session_id=<id>)` to read another agent's conversation, or `SessionSearch(action='search', query=<term>)` to find specific topics across all sessions
3. **Communicate** — Agent calls `message` with a target session ID, message text, and optional `context` array referencing specific turns from any session's history
4. **Verify** — Agent calls SessionSearch again to check that the target agent received and acted on the information

### 5.4 Bidirectional Messaging

Messaging is not limited to the existing supervisor→subordinate injection direction:
- Subordinate can message its supervisors
- Supervisor can message its subordinate
- Any agent can message any agent it knows about

The `message` action abstracts the routing direction — callers just specify a target `session_id` and message. Routing is handled internally based on the ChainOfCommand relationship between sender and receiver.

For supervisor→subordinate: routes through the existing `supervisor_input_tx` mpsc channel.
For subordinate→supervisor: routes through the supervisor session's input mechanism.

### 5.5 Mutual Awareness on Spawn

When a supervisor is spawned via the `spawn` action, both agents are informed of each other's session IDs:
- The **supervisor** receives the subordinate's session ID in its initial context
- The **subordinate** receives a notification that a new supervisor has been attached, including its session ID and role

This enables both to immediately use SessionSearch and message for coordination — no discovery step needed for agents that were just spawned together.

### 5.6 Ambient Trust

There are no access control boundaries between sessions within a project. Any agent can:
- List all sessions
- Inspect any session's full history via SessionSearch
- Message any session it knows about

Trust is ambient within a project.

### 5.7 Peer-to-Peer Coordination (Supervisor-to-Supervisor)

Multiple supervisors attached to the same subordinate can coordinate with each other without the subordinate mediating:

```
Subordinate spawns:
  ├── security-reviewer (supervisor)
  └── test-writer (supervisor)

test-writer calls:
  1. AgentManager(action='list') → discovers security-reviewer's session ID
  2. SessionSearch(action='show', session_id='<reviewer-id>') → reads security findings
  3. Writes tests covering the specific vulnerabilities found
  — no messages to subordinate needed
```

---

## 6. Coordination Examples

### 6.1 Full Coordination Loop
Agent spawns a security-reviewer, later checks its findings, and directs further investigation:
1. `AgentManager(action='spawn', role='security-reviewer', brief='Review all code changes for vulnerabilities')` → `{ session_id: 'reviewer-1' }`
2. `AgentManager(action='list')` → gets reviewer-1's session ID
3. `SessionSearch(action='search', session_id='reviewer-1', query='vulnerability')` → finds matches
4. `SessionSearch(action='show', session_id='reviewer-1', max_turns=5)` → reads full context
5. `AgentManager(action='message', session_id='reviewer-1', message='What about the new endpoint at /api/admin?')`

### 6.2 Supervisor Grounding Communication in History
Security-reviewer finds context from subordinate's past conversation and sends a message with references:
1. `SessionSearch(action='search', query='database migration')` → finds subordinate discussed a schema change in turns 42-44
2. `AgentManager(action='message', session_id='<subordinate-id>', message='The migration at line 45 drops a column still referenced by the User model', context=[{session_id: '<subordinate-id>', turns: [42, 43, 44]}])`
3. Subordinate receives the message with turns 42-44 inlined — immediately sees the prior discussion

### 6.3 Cross-Session Context in a Single Message
Test-writer references findings from both the security reviewer and its own work:
1. `AgentManager(action='message', session_id='<subordinate-id>', message='I have written regression tests for these two vulnerabilities', context=[{session_id: '<security-reviewer-id>', query: 'SQL injection'}, {session_id: '<test-writer-id>', turns: [8, 9]}])`
2. Subordinate gets the message with both the original security findings AND the test code inlined

### 6.4 Bidirectional Discussion
Subordinate disagrees with a supervisor's assessment, reads its reasoning, and pushes back:
1. Subordinate receives injection from security-reviewer
2. `SessionSearch(action='show', session_id='<reviewer-id>', max_turns=10)` → understands reviewer's reasoning
3. `AgentManager(action='message', session_id='<reviewer-id>', message='That endpoint is behind admin middleware — see line 23 of auth.ts', context=[{session_id: '<subordinate-id>', turns: [15]}])`

---

## 7. Relationship to Other Tools

```
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ SessionSearch │    │ AgentManager │    │  DeepSearch   │
│  (AMGR-001)  │    │  (AMGR-002)  │    │  (RLM-001)   │
│              │    │              │    │              │
│ • recent     │    │ • spawn      │    │ • query      │
│ • search     │    │ • set_role   │    │ • scope      │
│ • show       │    │ • list       │    │              │
│              │    │ • get_status │    │ Ephemeral    │
│ INSPECTION   │    │ • message    │    │ sub-agent    │
│ Cross-session│◄───│ • close      │    │              │
│ history read │ IDs│              │    │              │
└──────────────┘    └──────────────┘    └──────────────┘
       │                   │                    │
       │              session IDs               │
       │              flow between              │
       │              the two tools             │
       │                                        │
   INSPECTION          IDENTITY +          CODE SEARCH
   (read history)    COMMUNICATION         (read files)
                    (manage agents,
                     send messages
                     with context)
```

**SessionSearch requires no modifications** — it already accepts arbitrary `session_id` on its `show` action (any UUID, no access control), and `search` already returns matches across all sessions in the project with `session_id` in each result.

---

## 8. Migration Path from TUI-only to AgentManager

The supervisor system is **not being removed** — AgentManager wraps it. The TUI supervisor overlay (`/supervisor`) continues to work alongside the tool call. Over time, as agents learn to use AgentManager, the TUI-only path becomes less central. The key shift is:

| Before (TUI-only) | After (AgentManager) |
|---|---|
| Only user can create supervisors (TUI) | Agent can spawn supervisor agents (tool call) |
| Role set at creation, rarely changed | Role can be set/changed anytime |
| Templates stored in user config | Agent decides role parameters dynamically |
| Manual supervisor management | Agent manages its own supervisor ecosystem |
| Agents can't message each other | Agents message freely with context references |
| Agent must search other sessions itself | Sender can attach referenced context for receiver |

---

## 9. Key Design Decisions

1. **Single tool, action dispatch** — Same pattern as SessionSearch, Bridge
2. **Reuse existing infrastructure** — No parallel supervisor implementation
3. **No conversation forking** — N independent linear conversations + read access + messaging
4. **spawn defaults to caller's model** — Agent doesn't need to know model IDs
5. **auto_inject defaults to true** — Matches current supervisor behavior
6. **message uses existing injection channel** — SupervisorInput mpsc, formatted with `[SUPERVISOR: role | Session: id]` prefix
7. **message context is optional** — Plain strings for simple directives, context array for grounded communication
8. **Context resolved at send time** — Receiver gets self-contained message, doesn't redo sender's search work
9. **Bidirectional messaging** — Not limited to supervisor→subordinate direction
10. **close cascades cleanup** — ChainOfCommand removal, broadcast unsubscribe
11. **No authority parameter** — Removed by WATCH-024; the brief field provides all behavioral instruction
12. **Ambient trust** — No access control between sessions within a project
13. **Composition over coupling** — AgentManager and SessionSearch are independent tools that compose via session IDs
