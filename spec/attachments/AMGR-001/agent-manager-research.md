# AgentManager Tool — Research & Architecture Analysis

**Date:** 2026-03-08  
**Work Unit:** AMGR-001 (parent)  
**Author:** AI assistant + rquast  
**Status:** Discovery / Research Phase

---

## 0. Motivation

Today, agent orchestration in codelet is scattered across multiple subsystems:

| Capability | Current Implementation | Problem |
|---|---|---|
| **Session history search** | `scripts/session-search.sh` (760 lines of bash+Python) + `scripts/session-search-skill.md` (89 lines) | External script, slow, fragile, Python dependency, not available as a tool call |
| **Watcher agents** | `SessionRole`, `WatchGraph`, `ObservationBuffer` in `session_manager.rs` | Deep coupling to NAPI/TUI, not accessible from within a running agent |
| **Deep search sub-agents** | RLM-001 (specifying) — planned as standalone `DeepSearchTool` | Narrow: only read-only search, could be a specialization of general agent spawning |
| **Cross-agent communication** | `WatcherInput`, `watcher_broadcast` channels | Only watcher→parent injection, no peer-to-peer messaging |
| **Agent roles** | `SessionRole { name, description, authority, auto_inject }` | Limited to watchers, not composable with other agent types |

The **AgentManager** tool consolidates all of these into a single tool call that agents can use to:
1. Search their own (or other sessions') conversation history
2. Spawn sub-agents with specific roles and tool restrictions
3. Communicate with other active agents
4. List and manage running agents

---

## 1. Existing Infrastructure to Reuse

### 1.1 Persistence Layer (Session History)

The Rust persistence layer already has everything needed for session search:

```
codelet/napi/src/persistence/
├── storage.rs      — MessageStore: JSONL message storage with in-memory cache
├── history.rs      — HistoryStore: user input history with search()
├── blob.rs         — BlobStore: SHA-256 hashed content for large messages
├── blob_processing.rs — Blob reference resolution
├── types.rs        — StoredMessage, HistoryEntry, SessionData
├── message_envelope.rs — Message envelope format
└── napi_bindings.rs    — NAPI exports (we'd bypass these)
```

**Key APIs already in Rust:**
- `HistoryStore::search(query, project)` → search user input history
- `HistoryStore::get(project, limit)` → recent entries
- `MessageStore::get(id)` → retrieve message by UUID
- `BlobStore::resolve(ref)` → resolve `blob:sha256:HASH` references
- `load_session(session_id)` → load session metadata + message IDs

The bash script's `cmd_search`, `cmd_recent`, `cmd_show`, `cmd_context` commands can ALL be reimplemented using these Rust primitives directly, eliminating the Python dependency and subprocess overhead.

### 1.2 Session Manager (Agent Lifecycle)

```rust
// codelet/napi/src/session_manager.rs
pub struct SessionManager {
    sessions: IndexMap<Uuid, Arc<BackgroundSession>>,
    watch_graph: WatchGraph,
    // ...
}
```

The `SessionManager` already handles:
- Session creation (`create_session`, `create_watcher`)
- Session destruction (`destroy_session`)  
- Session listing (`list_sessions`)
- Attach/detach lifecycle
- Watcher registration via `WatchGraph`

### 1.3 Agent Construction (from RLM-001 research)

```rust
// Verified pattern from deep-search-architecture-research.md
let provider = manager.get_claude()?;
let rig_agent = provider.client()
    .agent(provider.model())
    .tool(ReadTool::new(session_id))
    .tool(GrepTool::new(session_id))
    // ... scoped tools only
    .preamble(&system_prompt)
    .build();
let agent = RigAgent::new(rig_agent, max_depth);
let result = agent.prompt(query).await?;
```

### 1.4 Role & Authority Model

```rust
pub struct SessionRole {
    pub name: String,
    pub description: Option<String>,
    pub authority: RoleAuthority,  // Supervisor | Peer
    pub auto_inject: bool,
}
```

### 1.5 Communication Channels

```rust
// Watcher broadcast — parent → watchers
watcher_broadcast: broadcast::Sender<StreamChunk>,

// Watcher injection — watcher → parent
pub struct WatcherInput {
    pub source_session_id: String,
    pub role_name: String,
    pub authority: RoleAuthority,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}
```

---

## 2. Proposed AgentManager Tool Design

### 2.1 Tool Actions (Discriminated Union)

The AgentManager tool uses an `action` field to dispatch, similar to Bridge and ConnectMCP:

```rust
pub struct AgentManagerArgs {
    action: AgentManagerAction,
}

pub enum AgentManagerAction {
    // --- Session History Search ---
    SearchHistory {
        query: String,
        scope: Option<SearchScope>,  // current, all, specific session IDs
        limit: Option<usize>,
    },
    ShowSession {
        session_id: String,
        user_only: Option<bool>,
        limit: Option<usize>,  // max turns to show
    },
    RecentSessions {
        count: Option<usize>,
        project: Option<String>,  // filter by project path
    },
    ContextSearch {
        keyword: String,
        max_turns: Option<usize>,
        last_n: Option<usize>,  // search last N history entries
    },

    // --- Agent Lifecycle ---
    SpawnAgent {
        role: String,           // e.g., "researcher", "reviewer", "coder"
        brief: String,          // system prompt / watching brief
        tools: Option<Vec<String>>,  // tool allowlist (None = all tools)
        model: Option<String>,  // override model selection
        parent: Option<String>, // parent session ID for watcher relationship
    },
    CloseAgent {
        agent_id: String,
    },
    ListAgents {},

    // --- Cross-Agent Communication ---
    SendMessage {
        target: String,         // agent_id or "parent" or "all"
        message: String,
        priority: Option<MessagePriority>,
    },
    ReadMessages {
        from: Option<String>,   // filter by source agent
        since: Option<String>,  // ISO timestamp
    },

    // --- Scoped Search (DeepSearch specialization) ---
    DeepSearch {
        query: String,
        scope: Vec<String>,     // paths, globs, session IDs
        model: Option<String>,
        max_depth: Option<usize>,
    },
}
```

### 2.2 Relationship to Existing Tools

| Existing Tool/Feature | AgentManager Action | Migration Path |
|---|---|---|
| `scripts/session-search.sh search` | `SearchHistory` | Replace entirely — Rust persistence APIs |
| `scripts/session-search.sh recent` | `RecentSessions` | Replace entirely |
| `scripts/session-search.sh show` | `ShowSession` | Replace entirely |
| `scripts/session-search.sh context` | `ContextSearch` | Replace entirely |
| `scripts/session-search-skill.md` | Embedded in tool description | Delete skill file |
| RLM-001 DeepSearch | `DeepSearch` | Subsume as specialization |
| WATCH-001 watcher creation | `SpawnAgent` | Unified interface |
| WATCH-006 watcher injection | `SendMessage` | Generalized to peer-to-peer |
| `WatchGraph` navigation | `ListAgents` | Expose as tool result |

### 2.3 Why a Single Tool vs Multiple Tools

**Single tool (AgentManager) with actions** — same pattern as:
- `Bridge` tool: `connect`, `disconnect`, `list` actions
- `ConnectMCP` tool: `connect`, `disconnect`, `list` actions
- `Fspec` tool: 100+ commands via single tool

Benefits:
- One tool definition for the LLM to learn
- Actions are naturally discoverable (the LLM sees all options in schema)
- Shared state (the agent manager has global view of all sessions)
- Consistent error handling and output format

---

## 3. Implementation Architecture

### 3.1 Module Structure

```
codelet/tools/src/
├── agent_manager/
│   ├── mod.rs              — AgentManagerTool struct + Tool impl
│   ├── session_search.rs   — SearchHistory, ShowSession, RecentSessions, ContextSearch
│   ├── lifecycle.rs        — SpawnAgent, CloseAgent, ListAgents
│   ├── messaging.rs        — SendMessage, ReadMessages
│   ├── deep_search.rs      — DeepSearch (from RLM-001 research)
│   └── types.rs            — AgentManagerArgs, SearchScope, etc.
```

### 3.2 Session Search Implementation

The key insight: `session-search.sh` does 4 things, all of which have Rust equivalents:

| Bash Script Function | Rust Equivalent |
|---|---|
| `cmd_search` — grep history.jsonl for keyword | `HistoryStore::search(query, project)` |
| `cmd_recent` — list recent session JSON files | `SessionStore::list_sessions(limit)` + `HistoryStore::get()` for summaries |
| `cmd_show` — resolve message IDs → content → blob refs | `MessageStore::get(id)` + `BlobStore::resolve(ref)` + reassembly logic |
| `cmd_context` — search + show matching turns with context | Combination of above |

The Python reassembly logic (200+ lines in the bash script) for reassembling streaming chunks back into readable text needs to be ported to Rust. The format is:
- `[Thinking: <first 50 chars>...]` — thinking chunks (often split mid-word, no closing bracket)
- `[Tool: <name>]` — tool invocations
- `[tool_result: ...]` — tool results
- Raw text fragments — response text split by SSE streaming

This reassembly is a pure string-processing function, straightforward to port.

### 3.3 Agent Spawning Architecture

Two modes of agent spawning:

**Mode A: Ephemeral sub-agent (DeepSearch pattern)**
- No session persistence
- `RigAgent::prompt()` — blocking, returns answer string
- Fresh `Uuid::new_v4()` for tool session_id
- Read-only tools only
- Result returned as tool output

**Mode B: Background agent (Watcher pattern)**  
- Full `BackgroundSession` with persistence
- Registered in `WatchGraph` if watching a parent
- Has its own streaming loop, message history
- Can be interacted with via `SendMessage`
- Listed by `ListAgents`, closed by `CloseAgent`

The `SpawnAgent` action decides the mode based on presence of `parent` and tool restrictions:
- No parent + restricted tools → Mode A (ephemeral, blocking)
- Has parent → Mode B (background watcher)
- No parent + full tools → Mode B (independent background agent)

### 3.4 Cross-Agent Messaging

Currently, watcher→parent injection uses `WatcherInput` which goes through the session's input channel. This can be generalized:

```rust
pub struct AgentMessage {
    pub source_id: Uuid,
    pub source_role: String,
    pub target_id: Uuid,  // or broadcast
    pub content: String,
    pub priority: MessagePriority,
    pub timestamp: DateTime<Utc>,
}

pub enum MessagePriority {
    Normal,     // Queued for next evaluation point
    Urgent,     // Interrupts current processing
}
```

A global message bus (or per-session inbox) allows any agent to send to any other agent.

---

## 4. Session Search — Data Model Deep Dive

### 4.1 File Layout (what session-search.sh reads)

```
~/.fspec/
├── sessions/
│   ├── {uuid}.json         — Session metadata + message_id array
│   └── ...
├── messages/
│   └── messages.jsonl      — All messages across all sessions
├── blobs/
│   ├── XX/                 — First 2 chars of SHA-256
│   │   └── {full-hash}    — Large content stored by hash
│   └── ...
└── history.jsonl           — User input display log with timestamps
```

### 4.2 Session JSON Structure

```json
{
  "id": "uuid",
  "name": "session name",
  "project": "/path/to/project",
  "provider": "claude",
  "created_at": "ISO8601",
  "updated_at": "ISO8601",
  "messages": [
    { "message_id": "uuid", "role": "user" },
    { "message_id": "uuid", "role": "assistant" }
  ],
  "compaction": { "compacted": false }
}
```

### 4.3 Message JSONL Structure

```json
{
  "id": "uuid",
  "role": "user|assistant",
  "content": "text or blob:sha256:HASH or [{type: 'text', text: '...'}]",
  "created_at": "ISO8601",
  "content_hash": "sha256",
  "token_count": 123,
  "metadata": {}
}
```

### 4.4 Blob Resolution

Content > threshold → stored as `blob:sha256:{hash}` in messages.jsonl, actual content in `~/.fspec/blobs/{hash[0:2]}/{hash}`.

The Rust `BlobStore` already handles this with `resolve()`.

---

## 5. Proposed Child Stories

### AMGR-002: Session History Search
**Scope:** `SearchHistory`, `ShowSession`, `RecentSessions`, `ContextSearch` actions  
**Replaces:** `scripts/session-search.sh`, `scripts/session-search-skill.md`  
**Estimate:** 5-8 points  
**Key work:**
- Port Python reassembly logic to Rust
- Wire persistence layer APIs into tool call handler
- Format results as readable tool output
- Support scope filtering (current project, all, specific sessions)

### AMGR-003: Agent Lifecycle Management  
**Scope:** `SpawnAgent`, `CloseAgent`, `ListAgents` actions  
**Depends on:** Agent construction pattern from RLM-001 research  
**Estimate:** 8 points  
**Key work:**
- Ephemeral sub-agent spawning (Mode A — DeepSearch)
- Background agent spawning (Mode B — watcher/independent)
- Tool restriction by allowlist
- Model override for cheaper sub-agents
- Session cleanup on close

### AMGR-004: Cross-Agent Communication
**Scope:** `SendMessage`, `ReadMessages` actions  
**Depends on:** AMGR-003  
**Estimate:** 5 points  
**Key work:**
- Message bus or per-session inbox
- Priority levels (normal, urgent)
- Message routing (direct, broadcast, parent)
- Integration with existing `WatcherInput` path

### AMGR-005: Scoped Deep Search
**Scope:** `DeepSearch` action (subsumes RLM-001)  
**Depends on:** AMGR-003 (agent spawning) + AMGR-002 (session history access)  
**Estimate:** 5 points  
**Key work:**
- Scope description generation
- RLM-adapted system prompt
- Session history serialization for search scope
- Tool scoping (Read, Grep, AstGrep, Glob, Ls, Bash only)

### AMGR-006: AgentManager Tool Shell  
**Scope:** Tool definition, parameter schema, action dispatch  
**Estimate:** 3 points  
**Key work:**
- `AgentManagerTool` struct implementing `rig::tool::Tool`
- JSON schema for all actions
- Action dispatch to sub-modules
- Wire into all providers' `create_rig_agent()`

---

## 6. Relationship to Existing Work Units

### RLM-001 (Deep Search Tool)
- **Current status:** Specifying (77 hours)
- **Relationship:** AMGR-005 subsumes RLM-001's scope
- **Migration:** RLM-001 becomes a child of AMGR-001, or we close it and reference AMGR-005
- **RLM-001's attachments** (RLM.md, deep-search-architecture-research.md) are highly relevant and should be referenced from AMGR-005

### WATCH-001 (Watcher Sessions MVP)
- **Current status:** Specifying (1110 hours!)
- **Relationship:** AMGR-003/004 overlap with WATCH-001's Rust-side agent spawning and communication
- **Key difference:** WATCH-001 is heavily TUI-focused (split views, purple text, overlays). The AgentManager tool is the **API layer** that WATCH-001's TUI would call.
- **Migration:** WATCH-001 remains as the TUI integration story. AgentManager provides the underlying tool API.

### Bridge Tool
- **No conflict:** Bridge is for external WebSocket endpoints (Telegram, etc.)
- **Synergy:** A background agent spawned via AgentManager could have Bridge connections

---

## 7. Phased Delivery Plan

### Phase 1: Session Search (AMGR-002 + AMGR-006 shell)
**Why first:** Highest immediate value. Every session currently uses the bash script. Replacing it with a native tool call is:
- Faster (no Python subprocess)
- More reliable (no script path resolution)
- Available to the agent as a tool (no Bash workaround)
- Eliminates the skill file dependency

### Phase 2: Agent Spawning (AMGR-003)
**Why second:** Enables the DeepSearch and watcher patterns. Once agents can spawn sub-agents, the entire agentic workflow opens up.

### Phase 3: Deep Search + Messaging (AMGR-004 + AMGR-005)
**Why third:** Builds on spawning. DeepSearch is a specialization of spawning. Messaging is needed for multi-agent workflows.

---

## 8. Open Questions for Discussion

1. **Should AgentManager be one tool or split into 2-3?**
   - Option A: Single `AgentManager` tool with action discriminator (like Bridge)
   - Option B: `SessionSearch` tool + `AgentManager` tool (search is distinct from orchestration)
   - Option C: `SessionSearch` + `AgentSpawn` + `AgentMessage` (fine-grained)

2. **How does the tool access the SessionManager singleton?**
   - Tools currently have `session_id: Uuid` but no reference to the global `SessionManager`
   - Options: thread-local, static global, passed via tool constructor, or message channel

3. **What happens to RLM-001?**
   - Option A: Close RLM-001 and reference AMGR-005
   - Option B: Make RLM-001 a child of AMGR-001
   - Option C: Keep RLM-001 independent, share code

4. **Background agent streaming — who sees the output?**
   - Ephemeral agents: output is returned as tool result (no streaming)
   - Background agents: output goes to... where? Watcher broadcast? A log? The TUI?

5. **Max concurrent agents?**
   - Current `MAX_SESSIONS = 10` includes watchers
   - Should spawned agents count against this limit?
   - Should there be a separate limit for ephemeral sub-agents?

---

## References

- `scripts/session-search.sh` — Current bash session search implementation
- `scripts/session-search-skill.md` — Agent skill file for session search
- `spec/attachments/RLM-001/deep-search-architecture-research.md` — DeepSearch architecture (verified 2026-03-05)
- `spec/attachments/RLM-001/RLM.md` — RLM paper analysis
- `codelet/napi/src/persistence/` — Rust persistence layer
- `codelet/napi/src/session_manager.rs` — SessionManager, WatchGraph, SessionRole
- `codelet/tools/src/bridge.rs` — Bridge tool pattern (action-based tool)
- `codelet/tools/src/mcp.rs` — ConnectMCP tool pattern
