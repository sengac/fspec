# AST Research — RPC-416 Inline reconnect status in scrollback

AST-based analysis (AstGrep, language: rust) of the existing surfaces the
inline-reconnect feature must reuse, delete, or extend. Confirms the design
doc's file/line references against the live tree before writing tests.

## Scrollback push / mutate / remove surfaces (to reuse)

Pattern: `pub fn push_line<S: Into<String>>(&mut self, line: S) { $$$BODY }`
- `src/store/agent_view/session_context.rs:159` — `SessionContext::push_line`
  builds a `ChunkKind::Notification` chunk and appends via `push_source`
  (allocates the stable `seq`). This is the push path for the inline line.

Pattern: `pub fn chunks_mut(&mut self) -> &mut Vec<RenderedChunk> { $$$BODY }`
- `src/views/agent/scrollback.rs:102` — mutable chunk vec access. The
  net-new `replace_by_seq` / `remove_by_seq` helpers resolve a `seq` to an
  index (linear scan) then edit `source.text` + `rewrap_at`, or `remove`.

## Per-session routing surfaces (to reuse for originating-session targeting)

Pattern: `pub fn session_context_mut_for(&mut self, id: &SessionId) -> Option<&mut SessionContext> { $$$BODY }`
- `src/store/agent_view.rs:141` — resolve a `SessionContext` by explicit
  `SessionId` (mirrors `EmitSessionNotice` silent-no-op-if-gone). The tracked
  `(SessionId, seq)` uses this so replace/remove target the ORIGINATING
  session regardless of current focus.

## Reconnect handler (to extend, not duplicate)

Pattern: `pub(crate) fn handle_reconnected(&mut self) { $$$BODY }`
- `src/app/dispatch_reconnect.rs:28` — RPC-415 respawn handler. Extend here to
  replace the tracked inline line with the success line and arm the
  tokio::sleep -> ClearReconnectNotice auto-dismiss timer. Keep the existing
  subscriber respawn + one-shot list_work_units/create_session re-bootstrap.

## Modal wiring (to delete)

Pattern: `pub struct DisconnectDialog { $$$FIELDS }`
- `src/components/disconnect_dialog.rs:24` — the modal component to remove
  entirely. Its push site is the `Action::Disconnected` guard arm in
  `src/app/dispatch.rs:39-41`; its removal site is `handle_reconnected`
  (`dispatch_reconnect.rs:29`, `compositor.remove(DISCONNECT_DIALOG_ID)`).
  Grep confirms `DISCONNECT_DIALOG_ID` referenced from dispatch.rs:3,
  dispatch_reconnect.rs:10, and the RPC-011 test.

## Conclusion

All design-doc anchors verified present. The inline feature reuses
`push_line` (push), adds seq-keyed `replace_by_seq`/`remove_by_seq` on
`ScrollbackList`, routes by `session_context_mut_for`, extends
`handle_reconnected`, and deletes `DisconnectDialog` + its two wiring sites.
No duplicated scrollback-mutation logic is introduced.
