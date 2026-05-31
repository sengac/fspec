# RPC-061 — AST research: supervisor / subordinate surface

**Card:** RPC-061 · **Phase:** 7.8 of RPC-030 · **Date:** 2026-05-24

This is the AST research dossier captured during DISCOVERY for RPC-061
(Supervisor / subordinate links surface). The goal of this card is to
add the supervisor surface to `SessionManagerHandle` + `FspecService` +
both backends + the production `SessionManager` impl + the
AgentView UI, with full cross-transport parity tests. Everything below
is concrete prior art on the Rust side already in the codebase.

---

## 1. Existing supervisor surface in `codelet-sessions`

### 1.1 `ChainOfCommand` (`codelet/sessions/src/chain_of_command.rs`)

```rust
pub struct ChainOfCommand {
    subordinate_to_supervisors: RwLock<HashMap<Uuid, Vec<Uuid>>>,
    supervisor_to_subordinates: RwLock<HashMap<Uuid, Vec<Uuid>>>,
}

impl ChainOfCommand {
    pub fn new() -> Self;
    pub fn add_supervisor(&self, subordinate_id: Uuid, supervisor_id: Uuid)
        -> Result<(), String>;            // cycle + duplicate detection
    pub fn remove_supervisor(&self, supervisor_id: Uuid);
    pub fn get_supervisors(&self, subordinate_id: Uuid) -> Vec<Uuid>;
    pub fn get_subordinate(&self, supervisor_id: Uuid) -> Option<Uuid>;
    pub fn get_subordinates(&self, supervisor_id: Uuid) -> Vec<Uuid>;
    pub fn cleanup_subordinate(&self, subordinate_id: Uuid);
    pub fn is_empty(&self) -> bool;
}
```

Cycle detection (line 55–73) is a BFS over `supervisor_to_subordinates`.
Returns `Err("circular supervision not allowed")` if the new supervisor
is already reachable from the new subordinate, and
`Err("subordinate already registered under this supervisor")` for
duplicate registrations.

### 1.2 `SessionManager` delegation (`codelet/sessions/src/session_manager.rs`)

```rust
// line 157
chain_of_command: ChainOfCommand,

// lines 948–971 — the existing delegation surface
pub fn add_supervisor(&self, subordinate_id: Uuid, supervisor_id: Uuid)
    -> Result<(), String>;
pub fn remove_supervisor(&self, supervisor_id: Uuid);
pub fn get_supervisors(&self, subordinate_id: Uuid) -> Vec<Uuid>;
pub fn get_subordinate(&self, supervisor_id: Uuid) -> Option<Uuid>;
pub fn get_subordinates(&self, supervisor_id: Uuid) -> Vec<Uuid>;
```

`SessionManager::destroy_session` (line 919) also calls
`chain_of_command.remove_supervisor(uuid)` so the supervisor map stays
consistent on session destruction.

### 1.3 `BackgroundSession::receive_incoming_message`
(`codelet/sessions/src/background_session.rs`)

```rust
// line 165
pub struct IncomingMessage {
    pub source_session_id: String,
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<BridgeImageData>>,
}

// line 1052
pub fn receive_incoming_message(&self, input: IncomingMessage)
    -> Result<(), String>;

// line 1071
pub fn pending_incoming_message_count(&self) -> usize;

// lines 348-354
incoming_message_tx: mpsc::Sender<IncomingMessage>,
pub incoming_message_rx: Mutex<mpsc::Receiver<IncomingMessage>>,
pub incoming_message_pending: Arc<AtomicUsize>,
```

The producer side increments `incoming_message_pending` after the
mpsc enqueue; the consumer side (agent loop) decrements on dequeue.

### 1.4 Supervisor broadcast (`background_session.rs:324, 801, 826`)

`supervisor_broadcast: broadcast::Sender<StreamChunk>` is used by
`handle_output` (line 801) to fan every output chunk into supervisors
that have subscribed. Out of scope for RPC-061 (subscription itself
belongs to a future card if needed) — but the stream chunk produced
by `receive_incoming_message` already includes
`StreamChunk::SupervisorPendingInjection`.

---

## 2. Existing surface in the trait + RPC layers

### 2.1 `SessionManagerHandle` (`codelet/core/src/session_manager_handle.rs`)

```rust
// line 376 — default no-op
fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId> {
    let _ = session_id;
    Vec::new()
}
```

Only `get_supervisors` exists today. `add_supervisor`,
`remove_supervisor`, `get_subordinate`, `get_subordinates`, and
`receive_incoming_message` MUST be added by this card.

`StubSessionManagerHandle` (line ~830) carries per-method AtomicU64
call counters and Mutex<HashMap<…>> seeded state for the existing
RPC-037..RPC-059 methods. The same pattern will be added for the
five new methods.

### 2.2 `FspecService` (`codelet/rpc/src/lib.rs`)

```rust
// line 282
async fn get_supervisors(session_id: SessionId) -> Vec<SessionId>;

// line 1228 — FspecServiceImpl routes through session_manager()
async fn get_supervisors(self, _ctx: Context, session_id: SessionId)
    -> Vec<SessionId>
{
    match self.inner.session_manager() {
        Some(handle) => handle.get_supervisors(&session_id),
        None => Vec::new(),
    }
}
```

Four new methods must be added to the tarpc service trait
(`add_supervisor`, `remove_supervisor`, `get_subordinate`,
`get_subordinates`, `receive_incoming_message`) plus matching
`FspecServiceImpl` routings.

### 2.3 `FspecBackend` trait (`codelet/fspec-tui/src/transport/mod.rs`)

```rust
// line 384 — default no-op
async fn get_supervisors(&self, _session_id: SessionId)
    -> Result<Vec<SessionId>> { Ok(Vec::new()) }
```

Only `get_supervisors` exists today. Five new methods must be added
to the trait (each with a no-op default for mock backends) plus
embedded + websocket forwarders.

---

## 3. Existing wire types (`codelet/rpc-types/src/lib.rs`)

### 3.1 `IncomingMessageImage` (line 705)

```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingMessageImage {
    pub data: String,
    #[serde(rename = "mediaType")]
    pub media_type: String,
}
```

Will be reused by the new `IncomingMessageInput` wire type without
modification.

### 3.2 `StreamChunk::IncomingMessage` / `SupervisorPendingInjection`
(`codelet/rpc-types/src/lib.rs:1047–1054`)

```rust
IncomingMessage {
    text: String,
    images: Option<Vec<IncomingMessageImage>>,
},
SupervisorPendingInjection {
    #[serde(rename = "supervisorPendingInjection")]
    supervisor_pending_injection: SupervisorPendingInjectionInfo,
},
```

Both variants already exist. `SupervisorPendingInjection` is what
`BackgroundSession.receive_incoming_message` emits via
`handle_output`, and it's the chunk the AgentView listens for to bump
its `supervisor_pending_count` slot.

### 3.3 `IncomingMessageInput` — **NEW** (this card)

Will be added next to `IncomingMessageImage`:

```rust
#[cfg_attr(feature = "napi", napi_derive::napi(object))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomingMessageInput {
    #[serde(rename = "sourceSessionId")]
    pub source_session_id: String,
    #[serde(rename = "roleName")]
    pub role_name: String,
    pub message: String,
    pub images: Option<Vec<IncomingMessageImage>>,
}
```

---

## 4. Existing chrome + dispatch surfaces

### 4.1 `SessionHeader` (`codelet/fspec-tui/src/views/agent/header.rs:45`)

```rust
pub struct SessionHeader<'a> {
    pub session_index: (usize, usize),
    pub model: Option<&'a ModelInfo>,
    pub thinking: ThinkingLevel,
    pub tokens: TokenState,
    pub work_unit_id: Option<&'a str>,
    pub work_unit_status: Option<&'a str>,
    pub is_isolated: bool,
    pub is_debug_enabled: bool,
    pub is_select_mode: bool,
    pub tokens_per_second: Option<f32>,
    pub reasoning_tokens: u64,
    pub compaction_reduction: Option<i32>,
    pub is_loading: bool,
}
```

Gains a `pub subordinate_label: Option<&'a str>` field this card.
`build_left_line` in `header_build.rs:15` is the place to append the
`[Subordinate of: <s>]` span (cyan, no modifier) after the [SELECT] /
[T:level] block.

### 4.2 `SessionFooter` (`codelet/fspec-tui/src/views/agent/footer.rs:40`)

```rust
pub struct SessionFooter<'a> {
    pub workspace: Option<&'a WorkspaceInfo>,
    pub compaction_progress: Option<&'a CompactionProgress>,
}
```

Gains a `pub supervisor_pending_count: usize` field this card.
`paint_left_aligned` (line 134) is the slot — supervisor chip wins
over compaction chip when both are non-zero.

### 4.3 `App::dispatch` pattern
(`codelet/fspec-tui/src/app/dispatch.rs:284–294`)

```rust
// RPC-022/050/053/054/056/057/058/060: route via try_dispatch_*
//   fallbacks so dispatch.rs stays under 300 LoC.
_ => {
    let _ = self.try_dispatch_rpc022(&action)
        || self.try_dispatch_rpc053(&action)
        || self.try_dispatch_rpc054(&action)
        || self.try_dispatch_rpc056(&action)
        || self.try_dispatch_rpc057(&action)
        || self.try_dispatch_rpc058(&action)
        || self.try_dispatch_rpc059(&action)
        || self.try_dispatch_rpc060(&action);
}
```

This card adds `try_dispatch_rpc061` after `try_dispatch_rpc060`.

### 4.4 `dispatch_rpc060.rs` reference pattern
(`codelet/fspec-tui/src/app/dispatch_rpc060.rs`)

Every recent card has its own `dispatch_rpcNNN.rs` helper module
containing `handle_*` methods and a `try_dispatch_rpcNNN(&Action)
-> bool` fallback. The RPC-060 file is 124 lines; the new
RPC-061 module follows the same shape and must also stay under 300
lines.

### 4.5 Stream-chunk handling
(`codelet/fspec-tui/src/app/dispatch_rpc045.rs`)

The `handle_stream_chunk_state_updates` function is the single place
that branches on `StreamChunk` variants for store updates. The
`StreamChunk::SupervisorPendingInjection` arm needs to bump
`store.supervisor_pending_count_for(...)` per chunk.

---

## 5. Existing AgentViewStore surface
(`codelet/fspec-tui/src/store/agent_view.rs`)

The store today has no `supervisors_for` or `supervisor_pending_count`
accessors. This card adds:

```rust
pub fn supervisors_for(&self, id: &SessionId) -> &[SessionId];
pub fn set_supervisors(&mut self, id: SessionId, supervisors: Vec<SessionId>);
pub fn supervisor_pending_count_for(&self, id: &SessionId) -> usize;
pub fn set_supervisor_pending_count(&mut self, id: SessionId, count: usize);
pub fn apply_supervisor_pending_injection(&mut self, id: &SessionId);
```

If `agent_view.rs` approaches the 300-LoC ceiling, a new sub-module
`agent_view/supervisor_state.rs` will hold the two HashMaps + helpers
(same pattern used by `chrome_state.rs`, `isolation_state.rs`, etc.).

---

## 6. MockBackend reference shape
(`codelet/fspec-tui/tests/common/mod.rs`)

Pattern from RPC-060 (lines 553–558):

```rust
/// RPC-060: seeded Result<IsolatedSessionInfo, String> returned by
/// create_isolated_session.
create_isolated_session_result:
    Mutex<std::result::Result<IsolatedSessionInfo, String>>,
/// RPC-060: per-call counter for create_isolated_session.
create_isolated_session_calls: AtomicUsize,
```

This card adds analogous fields per method.

---

## 7. End-to-end traceability of the RPC-061 surface

| Layer | File | Method(s) added |
|---|---|---|
| Wire type | `codelet/rpc-types/src/lib.rs` | `IncomingMessageInput` |
| Trait | `codelet/core/src/session_manager_handle.rs` | `add_supervisor`, `remove_supervisor`, `get_subordinate`, `get_subordinates`, `receive_incoming_message` |
| Stub | `codelet/core/src/session_manager_handle.rs::StubSessionManagerHandle` | All 5 + counters + cycle detection |
| Production handle | `codelet/sessions/src/handle_impl.rs` | All 5 (delegate to `ChainOfCommand` / `BackgroundSession`) |
| RPC service | `codelet/rpc/src/lib.rs::FspecService` | All 5 |
| RPC impl | `codelet/rpc/src/lib.rs::FspecServiceImpl` | All 5 (route through `session_manager()`) |
| Backend trait | `codelet/fspec-tui/src/transport/mod.rs::FspecBackend` | All 5 (with no-op defaults) |
| Embedded backend | `codelet/fspec-tui/src/transport/embedded.rs` | All 5 forwarders |
| WebSocket backend | `codelet/fspec-tui/src/transport/websocket.rs` | All 5 forwarders |
| AgentViewStore | `codelet/fspec-tui/src/store/agent_view.rs` | `supervisors_for`, `set_supervisors`, `supervisor_pending_count_for`, `set_supervisor_pending_count`, `apply_supervisor_pending_injection` |
| Action enum | `codelet/fspec-tui/src/components/mod.rs` | `SupervisorsLoaded(SessionId, Vec<SessionId>)`, `SendToSubordinate { subordinate_id, message }` |
| Dispatch helper | `codelet/fspec-tui/src/app/dispatch_rpc061.rs` (new) | `handle_supervisors_loaded`, `handle_send_to_subordinate`, `spawn_load_supervisors`, `try_dispatch_rpc061` |
| Dispatch wiring | `codelet/fspec-tui/src/app/dispatch.rs` | append `try_dispatch_rpc061` to fallback chain; `SessionCreated` arm spawns `spawn_load_supervisors` |
| Stream chunk | `codelet/fspec-tui/src/app/dispatch_rpc045.rs` | `SupervisorPendingInjection` arm bumps store |
| SessionHeader | `codelet/fspec-tui/src/views/agent/header.rs` + `header_build.rs` | `subordinate_label: Option<&'a str>` field + new span |
| SessionFooter | `codelet/fspec-tui/src/views/agent/footer.rs` | `supervisor_pending_count: usize` field + new chip |
| MockBackend | `codelet/fspec-tui/tests/common/mod.rs` | seeded results + counters for all 5 |
| Tests | `codelet/fspec-tui/tests/{source_shape_rpc061, rpc061_cross_transport_parity, supervisor_links_rpc061}.rs` | — |

---

## 8. Risks confirmed by the dive

- Loop rejection error string is `"circular supervision not allowed"` —
  the `ChainOfCommand` test expectation and the trait-level scenario
  MUST use that exact text.
- Duplicate registration error string is
  `"subordinate already registered under this supervisor"`.
- `BackgroundSession::receive_incoming_message` uses `try_send` on the
  bounded mpsc channel; when the queue is full or closed it returns
  `Err("Failed to queue supervisor input: <send error>")`. The Err
  message bubbles up unchanged through `SessionManager` and `FspecService`
  to the AgentView, which prepends `"[error] send to subordinate: "`
  in `EmitSessionNotice`.
- `IncomingMessage::new` rejects empty messages with
  `Err("message cannot be empty".to_string())`; `IncomingMessage::with_images`
  rejects empty *and* image-less inputs with
  `Err("message cannot be empty when no images are provided".to_string())`.
  The handle wrapper will surface those verbatim.

---

## 9. Out of scope (confirmed by the attachment)

- Auto-promotion of subordinates on supervisor disconnect.
- A new `/send <subordinate_id> <message>` slash command UX (the
  attachment leaves "Audit TS for the exact UX" to future scope).
  RPC-061 only adds the dispatch action `SendToSubordinate` and the
  trait + UI machinery the message would flow through; the
  slash-command entry point itself can land later.
