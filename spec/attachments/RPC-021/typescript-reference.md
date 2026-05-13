# RPC-021 — Multi-session support + Shift navigation + history

> **NOTE:** This card is large (likely 13+ points). Breaking it into 2–3
> children may be appropriate during the specifying phase. The breakdown
> proposal is at the bottom of this attachment.

## TypeScript reference

### Multi-session state
`src/tui/store/sessionStore.ts` + `src/tui/hooks/useSessionStreamManager.ts`

The TS TUI supports multiple concurrent AgentView sessions. The user can
cycle between them with `Shift+←` and `Shift+→`. The header `#N:` numbers
sessions in creation order; the current session is rendered with its
scrollback, while inactive sessions retain their state (input buffer,
scroll offset, model, role, etc.).

### Session cycling
`src/tui/components/AgentView.tsx` — search for `Shift+leftArrow` /
`Shift+rightArrow` handlers. They call `sessionStore.cycleSession(±1)`
which updates `currentSessionId` and the next render shows the other
session's state.

### Command history (Shift+↑ / Shift+↓)
Persistence-backed history of previously-submitted inputs. Keys:
- `Shift+↑` — recall the previous input, replacing the current draft.
- `Shift+↓` — advance to the next input (or empty if at the end).

Backed by NAPI:
- `persistenceGetHistory(sessionId, limit, offset)` → `string[]`
- `persistenceAddHistory(sessionId, text)` → `void`
- `persistenceSearchHistory(sessionId, query)` → `string[]`

See `src/tui/components/AgentView.tsx` imports at lines 67-71.

### Session resume picker (`/resume`)
Opens a list of all known sessions sorted by last-used timestamp.
Selecting one attaches to it via `attachToSession` (line 51-53). Backed
by `persistenceListSessions()`.

### History search (`/search`)
Opens a typeahead that searches across all sessions' history. Backed by
`persistenceSearchHistory`.

## Current Rust state

`AgentViewStore` (`codelet/fspec-tui/src/store/agent_view.rs`) has a
single `current_session: Option<SessionId>` field. No multi-session
support, no history, no session list.

`App::dispatch` for `Action::SessionCreated` (line 120-133) sets the
single current session.

## Target Rust behavior

### Major AgentViewStore extension

```rust
pub struct AgentViewStore {
    // existing fields kept...

    /// All known open sessions in the AgentView, in creation order.
    /// The current_session field above is one of these.
    open_sessions: Vec<SessionContext>,
    current_session_index: usize,    // index into open_sessions

    /// Per-session history navigation state (recall position).
    history_state_by_session: HashMap<SessionId, HistoryNavState>,
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub id: SessionId,
    pub work_unit_id: Option<String>,
    pub scrollback: ScrollbackList,    // from RPC-019
    pub input_draft: String,            // saved when switching sessions
}

#[derive(Debug, Clone, Default)]
pub struct HistoryNavState {
    pub recall_index: Option<usize>,   // None = current draft (live input)
    pub cached_draft: Option<String>,  // saved live input when recalling
}
```

### Switching sessions

```rust
impl AgentViewStore {
    pub fn cycle_session(&mut self, delta: isize) -> Option<&SessionContext> {
        if self.open_sessions.is_empty() { return None; }
        let len = self.open_sessions.len() as isize;
        let next = (self.current_session_index as isize + delta).rem_euclid(len) as usize;
        self.current_session_index = next;
        self.open_sessions.get(next)
    }
}
```

### New FspecBackend methods

```rust
#[async_trait]
pub trait FspecBackend: Send + Sync {
    // existing methods...

    /// RPC-021: append a submitted input to the session's history.
    async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>;

    /// RPC-021: return the most recent `limit` history entries for the session,
    /// newest-first.
    async fn persistence_get_history(
        &self,
        session: SessionId,
        limit: u32,
    ) -> Result<Vec<String>>;

    /// RPC-021: search history across all sessions matching `query`.
    async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>;
}

#[derive(Debug, Clone)]
pub struct HistoryMatch {
    pub session_id: SessionId,
    pub text: String,
    pub timestamp_iso: String,
}
```

### New RPC methods + shared types

Same shape, lifted to `codelet/rpc/src/lib.rs` + `codelet/rpc-types/src/lib.rs`.

### Service impl

Delegates to `codelet_core::persistence::history` module (likely already
exists since the NAPI surface uses it — confirm in specifying phase).

### NAPI surface preservation

The existing NAPI exports `persistenceGetHistory`, `persistenceAddHistory`,
`persistenceSearchHistory` are **kept exactly as-is**. The new RPC
methods are additive — they delegate to the same `codelet_core::persistence`
functions the existing NAPI exports already use.

### Input wiring (Shift+↑/↓ for history)

Extend `MultiLineInput` from RPC-019 to forward Shift+↑/↓ to the caller
as `Action::HistoryPrev` / `Action::HistoryNext`. App::dispatch:
1. On `HistoryPrev`: if `recall_index.is_none()` → cache the current draft,
   load history asynchronously, set `recall_index = Some(0)`, replace
   input value with `history[0]`.
2. On subsequent `HistoryPrev`: increment `recall_index`, replace input
   with `history[recall_index]` (clamped).
3. On `HistoryNext`: decrement `recall_index`. If it goes below 0 →
   set `recall_index = None` and restore cached draft.

### Input wiring (Shift+←/→ for sessions)

`Action::SessionPrev` / `SessionNext` → call `agent_view_store.cycle_session(±1)`,
update `active_session_tx` watch channel for the chunks subscriber, mark
render dirty.

### Submission persists history

`App::dispatch` for `Action::InputSubmitted(text)` (already in
`codelet/fspec-tui/src/app/dispatch.rs::handle_input_submitted`) now
also fires-and-forgets `backend.persistence_add_history(session, text)`.

## RPC/NAPI boundary contract

```
TS AgentView → persistenceGetHistory / persistenceAddHistory / persistenceSearchHistory
              → already wired, unchanged

Rust TUI → FspecBackend::persistence_get_history / _add / _search
       → FspecService::persistence_get_history / _add / _search [tarpc]
       → codelet_core::persistence::history::{get, add, search} [shared impl]
```

Same shared backend, both UIs stay in sync.

## Existing TypeScript behavior preserved

- All TS multi-session code in `src/tui/store/sessionStore.ts` — UNCHANGED.
- All TS history code in `src/tui/components/AgentView.tsx` — UNCHANGED.
- All TS persistence NAPI exports — UNCHANGED.

## Recommended breakdown into child cards

This card is 13+ points and should split during specifying:

- **RPC-021a (~5pts)** — Multi-session state + Shift+←/→ cycling. Just
  the `open_sessions` Vec and the cycling actions; no history.
- **RPC-021b (~5pts)** — Shift+↑/↓ history recall + history persistence
  RPC methods + NAPI integration.
- **RPC-021c (~3pts)** — `/resume` session picker + `/search` history
  search popups (the slash-command handlers from RPC-020 that depend on
  21a + 21b).

## Acceptance criteria sketch (parent card)

- The Rust AgentView supports multiple concurrent sessions accessible
  via Shift+←/→.
- Switching sessions preserves each session's scrollback, input draft,
  and scroll offset.
- The header `#N:` shows the current session's 1-based index among
  open sessions.
- Shift+↑ recalls the previous input from history.
- Shift+↓ advances toward the live draft.
- Submitting an input persists it to history via the new RPC method.
- `/resume` opens a session picker; selecting attaches to that session.
- `/search` opens a typeahead over all history; selecting inserts the
  text into the input.
- All existing TS multi-session and history behavior still works
  unchanged.
