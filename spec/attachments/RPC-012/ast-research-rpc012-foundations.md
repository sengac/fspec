# RPC-012 — AST Research

**Date:** 2026-05-13
**Phase:** Discovery → Specifying handoff
**Tool:** AstGrep over `codelet/fspec-tui/src/` (Rust) + `src/tui/` (TypeScript reference)

---

## 1. Existing `pub struct` surface in `codelet/fspec-tui/src/`

| File | Struct |
|---|---|
| `transport/embedded.rs` | `EmbeddedFspecBackend` |
| `transport/websocket.rs` | `WebSocketFspecBackend`, `SupervisorHandle` |
| `terminal.rs` | `TerminalGuard` |
| `theme.rs` | `Theme` |
| `compositor.rs` | `Compositor` |
| `components/help_dialog.rs` | `HelpDialog` |
| `components/hello.rs` | `HelloComponent` |
| `components/disconnect_dialog.rs` | `DisconnectDialog` |
| `app.rs` | `App` |
| `views/work_units_list.rs` | `WorkUnitsListView` *(to be removed)* |
| `views/root.rs` | `RootView` *(to be replaced by `Navigator`)* |
| `views/footer.rs` | `FooterView` *(kept)* |
| `views/agent_repl.rs` | `RenderedChunk`, `AgentReplView` *(`AgentReplView` to be replaced by `AgentView`; `RenderedChunk` migrates into `views/agent.rs`)* |

## 2. Existing `pub enum` surface

| File | Enum |
|---|---|
| `transport/mod.rs` | `BackendError` |
| `components/mod.rs` | `Priority`, `EventResult`, `Action` *(extended with 5 new variants)* |
| `views/root.rs` | `FocusedPane` *(removed alongside `RootView`)* |

## 3. New surfaces RPC-012 will introduce

- `store::BoardStore` (`codelet/fspec-tui/src/store/board.rs`)
- `store::AgentViewStore` (`codelet/fspec-tui/src/store/agent_view.rs`)
- `store::COLUMN_ORDER` const `[&str; 7]`
- `views::Navigator` (`codelet/fspec-tui/src/views/navigator.rs`)
- `views::ViewMode { Board, Agent }` enum
- `views::BoardView` (`codelet/fspec-tui/src/views/board.rs`)
- `views::AgentView` (`codelet/fspec-tui/src/views/agent.rs`) *(slim port of `AgentReplView`)*
- `Action` gains: `EnterWorkUnit(String)`, `OpenAgentView(Option<SessionId>)`, `BackToBoard`, `NavigationTargetSet(Option<SessionId>)`, `AttachSession(String, SessionId)`

## 4. TypeScript reference cross-check

- `src/tui/store/fspecStore.ts` — `sessionAttachments: Map<string, string>` is the source for `BoardStore.session_attachments: HashMap<String, SessionId>`.
- `src/tui/store/sessionStore.ts` — fields `currentSessionId`, `navigationTargetSessionId`, `currentWorkUnitId`, `currentWorkUnitStatus`, `showCreateSessionDialog`, `shouldAutoCreateSession` are the source for `AgentViewStore`.
- `src/tui/components/UnifiedBoardLayout.tsx` `STATES` array exactly matches the proposed Rust `COLUMN_ORDER`.
- `src/tui/components/AgentView.tsx` is large (5624 lines) — only the navigation-handoff and active-session-binding contracts are mirrored in this slice; multi-session/slash-commands/model-picker land in downstream slices.

## 5. Source-shape invariants to preserve

- No `Mutex` / `RwLock` / atomics in any `store/*.rs` public surface.
- No `tokio::runtime::Builder` or `Runtime::new` in any `store/*.rs` file.
- No imports of `codelet_napi::`, `codelet_core::`, `tarpc::`, `tokio_tungstenite::` from `store/*.rs`.
- Every new module under `codelet/fspec-tui/src/` stays < 300 LoC.

## 6. RPC-009 invariants carried forward

- Action bus mutation happens only on the App task.
- Subscriber tasks spawned via `tokio::spawn` on host runtime `Handle` (RPC-005 Q9).
- Chunks subscriber filters via `watch::channel<Option<SessionId>>` — feed from `AgentViewStore.current_session` instead of `AgentReplView.active_session`.
