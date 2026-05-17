# RPC-026 AST Research — code shape that constrains the resume picker + search palette wiring

Generated during the Example Mapping phase of RPC-026 (`/resume`
session picker + `/search` history palette — RPC-021c). Captures the
AST-level shape of the existing patterns that this slice extends so the
implementation lands consistently with RPC-020, RPC-024, and RPC-025.

## Existing popup widgets (the pattern we are following)

```text
codelet/fspec-tui/src/views/agent/slash_command_popup.rs:45:1:pub struct SlashCommandPopup {
codelet/fspec-tui/src/views/agent/file_search_popup.rs:39:1:pub struct FileSearchPopup {
```

Both widgets:

- Own filter/query/matches/selected_index state.
- Expose `handle_key(code, mods) -> {…}Outcome`.
- Render via `tui_popup::Popup::new(...).title("...").render(area, buf)`.
- Use `super::popup_body::{widest_line, PopupBody}` for body layout.
- Are owned by `AgentView` as `Option<...>`.

We will mirror this shape for `ResumePicker` and `SearchPalette`.

## Action enum (where the seven new variants land)

```text
codelet/fspec-tui/src/components/mod.rs:86:1:pub enum Action {
```

The `Action` enum currently derives `#[derive(Debug, Clone)]` (no
`PartialEq` — `StreamChunk` doesn't derive it). The seven new variants
introduced by RPC-026 are all `Debug + Clone` compatible.

## Backend RPC methods we depend on (already landed)

```text
codelet/fspec-tui/src/transport/embedded.rs:61:5:  async fn list_sessions(&self) -> Result<Vec<SessionInfo>>
codelet/fspec-tui/src/transport/websocket.rs:185:5: async fn list_sessions(&self) -> Result<Vec<SessionInfo>>
codelet/fspec-tui/src/transport/embedded.rs:163:5: async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>
codelet/fspec-tui/src/transport/websocket.rs:372:5: async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>
```

Both backends already expose `list_sessions` (RPC-007) and
`persistence_search_history` (RPC-025) over identical shapes. RPC-026
does NOT add a single new method to either trait — only consumes them.

## Implications for RPC-026

1. **No new RPC method.** Both popups are pure consumers of methods
   that already exist on `FspecBackend`. We only add Action variants
   + widgets + dispatch wiring.
2. **`HistoryMatch` already lives in `codelet-rpc-types`** (RPC-025).
   We import it from `codelet_rpc_types::HistoryMatch`.
3. **Dispatch helpers** follow the RPC-025 split — App helpers live in
   `codelet/fspec-tui/src/app/dispatch_rpc026.rs` so `dispatch.rs`
   stays under 300 LoC.
4. **Cross-transport parity tests** follow the RPC-025 pattern of
   driving the same scenario against `EmbeddedFspecBackend` AND
   `WebSocketFspecBackend` via a loopback `bind_and_serve` daemon.

## Risks observed

- `SessionInfo` does NOT carry a `last_used_at` timestamp, so any
  "sorted by last used" UX would need a server-side sort. RPC-026
  defers that — we render the session list in the order the backend
  returns. Parent card RPC-021 description mentions "sorted-by-last-used"
  but that is upstream of this slice (would need a SessionInfo field
  + `list_sessions` ordering change which is out of scope for the
  popup widgets).
- `SessionInfo.id` is a `String`, not a `SessionId` newtype — the
  resume picker stores `SessionInfo` and converts to `SessionId` at
  Enter time via `SessionId::new(info.id.clone())`.
