# BUG-328 root cause: send_input succeeds for sessions the daemon does not hold

## Problem

- RPC declarations: `send_input(session_id, text)` returns `()`
  (`rust/rpc/src/service_declaration/workspace.rs:28`);
  `send_input_with_thinking` likewise
  (`rust/rpc/src/service_declaration/providers_sessions.rs:148`).
- The handle methods return `()` (`rust/core/src/session_manager_handle/handle_trait_1.rs:30`,
  `handle_trait_2.rs:51`); the sessions impl
  (`rust/sessions/src/handle_impl/handle_impl_1.rs:134-149`) calls
  `live_or_resumed` and discards `BackgroundSession::send_input`'s `Result`
  with `let _ =`.
- BUG-287 (commit fb6c3537) added `SessionManager::live_or_resumed`
  (`rust/sessions/src/session_manager/manager_impl_6.rs:9-22`): if the session
  is not live it silently calls `resume_session` from disk; for an unknown id it
  only logs. This is an implicit fallback — input to a session the client never
  resumed restores it as a side effect — and `submit_input` (RIG-016,
  `rust/sessions/src/handle_impl/handle_impl_5.rs:179`) uses it too.
- The TUI ignores the result: `rust/fspec-tui/src/app/dispatch_slash_submit.rs:163`
  `let _ = backend.send_input(..)`.

A client sending input to a destroyed or never-resumed session gets success and
waits for a stream that never comes (or, with BUG-287, gets an unrequested
resume).

## Design

- `send_input` and `send_input_with_thinking` return `Result<(), String>` on
  every layer: RPC declaration, service implementation, MCP dispatch, core
  `SessionManagerHandle` (and its stub), sessions handle impl, TUI
  `FspecBackend`, embedded and websocket transports, test mocks.
- Input is accepted only for a live session. A session the daemon does not
  hold is refused with `Session not found: <id> (resume it with resume_session
  before sending input)`. `live_or_resumed` is removed; `send_input`,
  `send_input_with_thinking` and `submit_input` look up the live session only.
- `BackgroundSession::send_input`'s error is propagated, not discarded.
- The TUI shows a failed send as an `[error] send failed: <reason>` line in the
  session's scrollback (existing `Action::EmitSessionNotice` mechanism) and logs
  it.
- BUG-287's scenarios become: input to a persisted-but-not-live session errors
  with the resume hint; input to an unknown session errors.

## Alternatives rejected

- Keeping BUG-287's implicit resume: a hidden fallback; resume has its own
  RPC, side effects (lease, replay, broadcasts) and failure modes the client
  should drive and observe explicitly.
- Keeping `()` and only logging: the client cannot know its input was lost.

## Wire / API changes

- `send_input(session_id: SessionId, text: String) -> Result<(), String>`
- `send_input_with_thinking(session_id: SessionId, text: String, thinking:
  Option<ThinkingConfig>) -> Result<(), String>`
- `submit_input` keeps its signature; it now errors (instead of resuming) for a
  session that is not live.
Method order is unchanged.
