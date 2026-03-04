# MCP Listener Design Options for Server-Initiated Input Injection

## Problem Statement

The agent currently supports **agent-initiated** external calls:
- The agent calls the `Bash` tool to run a script
- The script connects to an MCP server, invokes a tool, and returns the result
- The agent receives the result as tool output

This works because the agent controls the timing. But MCP servers can also **initiate** communication:
- **Notifications** (`notifications/resources/updated`, `notifications/tools/list_changed`)
- **Sampling requests** (`sampling/createMessage`) — the server asks the *client* to run an LLM prompt
- **Resource subscription updates** — pushed when subscribed resources change
- **Log messages** — server-initiated diagnostic output

For these server-initiated messages, there's no agent tool call in progress. The messages arrive asynchronously and need to be injected into the agent's active session — similar to how a **watcher session** observes and interjects into a parent session.

## Current Architecture (What We Can Build On)

### 1. Bridge Tool & Relay System (`codelet/tools/src/bridge.rs`, `bridge_relay.rs`)

The Bridge tool already provides a WebSocket-based I/O relay:
- **Outbound**: Session `StreamChunk`s are serialized as JSON and sent to an external WebSocket endpoint
- **Inbound**: Messages from the endpoint are parsed and injected via `InputInjector` (an `Arc<dyn Fn(InjectedInput) + Send + Sync>`)
- **Message types**: `input` (text/images), `control` (interrupt/clear/pause_response), `command` (fspec command execution)
- **Auto-reconnect** with exponential backoff
- **Per-session context** via `BridgeSessionContext` with broadcast receivers, input injectors, and control handlers

### 2. Watcher Sessions (`session_manager.rs`)

Watcher sessions provide a model for AI-mediated input injection:
- A watcher subscribes to a parent session's `broadcast` channel (receives all `StreamChunk`s)
- The watcher accumulates observations and periodically evaluates them through its own LLM
- If the watcher decides to interject, it uses `watcher_inject()` which creates a `WatcherInput` message and sends it via the parent's `watcher_input_tx` channel
- The parent's `agent_loop` receives watcher input via `tokio::select!` alongside user input, with user input taking priority (`biased`)
- Interjections appear in the UI with source attribution (role name, authority level)

### 3. Input Injection Channels

Every `BackgroundSession` has:
- `input_tx` / `input_rx`: Primary user input channel (`mpsc::Sender<PromptInput>`)
- `watcher_input_tx` / `watcher_input_rx`: Secondary injection channel for watchers/bridges (`mpsc::Sender<WatcherInput>`)
- `broadcast_tx`: Broadcast channel for outbound stream chunks

The `agent_loop` uses `tokio::select! { biased; }` to multiplex both channels, always preferring user input.

## Design Options

### Option A: Script Listener via Local WebSocket (Recommended)

**Concept**: The agent exposes a local WebSocket listener (per session) that external MCP proxy scripts can connect to for bidirectional communication.

```
┌──────────────┐    stdio     ┌──────────────┐   WebSocket   ┌──────────────┐
│  MCP Server  │◄────────────►│  MCP Proxy   │──────────────►│  Session     │
│  (external)  │              │  Script      │               │  Listener    │
│              │              │  (user code) │◄──────────────│  (agent)     │
└──────────────┘              └──────────────┘               └──────────────┘
                                                                    │
                                                                    ▼
                                                             watcher_input_tx
                                                             (inject into session)
```

**How it works:**
1. Agent starts a local WebSocket server on an ephemeral port, bound to `127.0.0.1`
2. The port is exposed to the agent and available in the session environment
3. User writes a script that:
   - Connects to the MCP server via stdio/SSE transport (standard MCP client)
   - Connects to the agent's local WebSocket listener
   - Forwards MCP server-initiated messages TO the agent's WebSocket
   - Receives agent responses/tool results FROM the WebSocket
4. The listener injects incoming messages into the session via `watcher_input_tx`
5. The agent sees them as user/watcher messages and can respond

**Message protocol** (reuses existing `InboundMessage` format from bridge_relay):
```json
// MCP notification → agent
{
  "type": "input",
  "session_id": "...",
  "message": "[MCP:my-server] Resource updated: file:///path/to/resource\n\nNew content: ..."
}

// MCP sampling request → agent
{
  "type": "input",
  "session_id": "...",
  "message": "[MCP:my-server] Sampling request:\n\nPlease analyze the following code and suggest improvements:\n```python\ndef foo():\n  ...\n```"
}
```

**Advantages:**
- Reuses existing `InboundMessage` format and `InputInjector` infrastructure
- No changes to the core agent loop needed — uses existing `watcher_input_tx`
- User scripts are fully decoupled — any language, any MCP transport
- Works with any MCP server without fspec needing to understand MCP protocol
- Local-only WebSocket is secure (bound to 127.0.0.1)
- Agent can call the script's MCP tools via `Bash` tool for agent-initiated calls, AND receive server pushes via the listener

**Disadvantages:**
- Requires a per-session WebSocket server (port management)
- User must write a bridge script (though we can provide templates)
- Two-hop latency for messages (MCP server → script → listener → session)

**Implementation scope:**
- New NAPI function: `start_session_listener(session_id) → port`
- New NAPI function: `stop_session_listener(session_id)`
- Listener uses same `InboundMessage` parsing as `bridge_relay.rs`
- Injects via existing `watcher_input_tx` channel
- Expose listener port in session environment info
- Provide MCP proxy script templates (Node.js, Python)

---

### Option B: Named Pipe / Unix Socket Listener

**Concept**: Instead of a WebSocket, use a Unix domain socket or named pipe per session. The external MCP script writes newline-delimited JSON to the pipe.

```
┌──────────────┐    stdio     ┌──────────────┐   Unix socket  ┌──────────────┐
│  MCP Server  │◄────────────►│  MCP Proxy   │───────────────►│  Session     │
│              │              │  Script      │                │  Listener    │
└──────────────┘              └──────────────┘                └──────────────┘
                                                                    │
                                                                    ▼
                                                             watcher_input_tx
```

**Socket path**: `/tmp/fspec-sessions/<session-id>.sock`

**Advantages:**
- No port management needed
- Slightly lower overhead than WebSocket
- Natural file-system permission model
- Works well on macOS/Linux

**Disadvantages:**
- Not cross-platform (Windows requires named pipes with different API)
- Less tooling support (WebSocket libraries are more universal)
- One-directional by default unless using stream sockets (no built-in response channel)
- Harder for user scripts in some languages

**Implementation scope:**
- Create Unix socket at session start
- Tokio `UnixListener` accepts connections
- Read newline-delimited JSON, parse as `InboundMessage`
- Inject via `watcher_input_tx`
- Clean up socket on session end

---

### Option C: Bridge Tool Extension (Reverse Bridge)

**Concept**: Extend the existing Bridge tool to support a "listen" action where the agent starts a WebSocket *server* instead of connecting as a *client*.

```
Agent calls:  Bridge(action: "listen", port: 9100)
              ──► Starts WS server on :9100
              ──► MCP proxy script connects as client
              ──► Inbound messages injected into session
```

**Advantages:**
- Reuses 100% of existing Bridge infrastructure (InboundMessage, InputInjector, ControlHandler)
- No new NAPI bindings needed — just a new action on the existing tool
- Agent explicitly controls when listening starts/stops
- Familiar pattern for AI agents already using Bridge

**Disadvantages:**
- Mixes client/server semantics in one tool (Bridge is currently client-only)
- Port conflicts if multiple sessions listen on same port
- Agent must explicitly call the tool (not automatic on session start)

**Implementation scope:**
- Add `listen` action to `BridgeAction` enum
- `BridgeManager` tracks both client connections and server listeners
- WebSocket server handler parses `InboundMessage`, calls `InputInjector`
- Add `stop_listening` action or extend `disconnect`

---

### Option D: Subprocess Listener with stdio Relay

**Concept**: The agent spawns the MCP proxy script as a subprocess and relays its stdout as injected input, stdin as session output.

```
┌──────────────┐    stdio     ┌──────────────┐    stdio    ┌──────────────┐
│  MCP Server  │◄────────────►│  MCP Proxy   │◄──────────►│  Agent       │
│              │              │  (subprocess)│            │  Session     │
└──────────────┘              └──────────────┘            └──────────────┘
```

**Advantages:**
- Simplest script authoring — just read/write stdio
- No network stack needed
- Process lifecycle tied to session
- Agent has full control (can kill subprocess)

**Disadvantages:**
- Only one MCP server per subprocess (can't easily multiplex)
- No reconnection semantics (process dies = gone)
- Script must handle MCP transport AND stdio framing
- Harder to debug (no separate process to inspect)
- Subprocess management adds complexity to session_manager.rs

---

### Option E: Hybrid — Bridge Listen + Watcher-Style AI Mediation

**Concept**: Combine Option C with a watcher-like AI layer. The listener not only injects raw messages but can optionally route them through a lightweight AI evaluation (like watchers do) to decide whether to inject, how to format, and whether it's urgent.

```
MCP Proxy ──► WS Listener ──► AI Evaluator (optional) ──► watcher_input_tx
                                     │
                                     ▼
                              "Is this worth interrupting
                               the agent for?"
```

**Advantages:**
- Intelligent filtering — not every MCP notification needs to interrupt the agent
- Can batch/summarize multiple notifications
- Familiar pattern (watchers already do this)

**Disadvantages:**
- Much more complex
- Uses LLM tokens for filtering
- Latency for urgent messages
- Probably over-engineered for V1

---

## Recommendation

**Option A (Local WebSocket Listener)** is recommended for V1 because:

1. **Minimal core changes** — Uses existing `watcher_input_tx` injection, existing `InboundMessage` parsing
2. **Maximum flexibility** — User scripts can be in any language, use any MCP transport
3. **Clean separation** — fspec doesn't need to understand MCP protocol at all
4. **Proven pattern** — Bridge relay already validates this WebSocket-based approach
5. **Cross-platform** — WebSockets work everywhere (unlike Unix sockets)

**Option C (Bridge Listen action)** is a strong alternative that could replace or complement Option A. It has the advantage of zero new NAPI bindings, but the downside of requiring the agent to explicitly call the Bridge tool to start listening.

**Recommendation for phased approach:**
1. **Phase 1**: Option A — Local WebSocket listener per session (automatic, always-on)
2. **Phase 2**: Provide official MCP proxy script templates (Node.js + Python)
3. **Phase 3**: Consider Option E's AI mediation layer if noise becomes a problem

## Key Questions to Resolve

1. **Should the listener start automatically** with every session, or only on-demand (via a tool call or slash command)?
2. **Should we provide an MCP proxy template** or just document the protocol and let users write their own?
3. **Message format**: Should we use the existing `InboundMessage` format exactly, or define a simpler MCP-specific format?
4. **Authentication**: Should the local listener require any auth token, or is binding to 127.0.0.1 sufficient?
5. **Multiple listeners per session**: Should a session support multiple concurrent listeners (one per MCP server)?
6. **Response channel**: For MCP sampling requests, the server expects a response. How should the agent's response flow back to the MCP proxy script? (Outbound broadcast → script filters for relevant chunks?)
7. **Resource discovery**: How does the agent know what MCP tools/resources are available from connected servers? (Script provides a manifest on connect? Agent discovers via a tool call?)
