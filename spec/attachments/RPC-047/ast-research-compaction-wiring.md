# RPC-047 — AST Research: /compact wiring targets

Source surveys performed via `AstGrep` and `Grep` over
`codelet/fspec-tui/src/**` to confirm the exact extension points
RPC-047 must touch. All findings inform the architecture notes that
already live on the work unit and the test layout in
`codelet/fspec-tui/tests/slash_compact_rpc047.rs`.

## SlashCommandAction::Compact — current binding

```
codelet/fspec-tui/src/views/agent/slash_commands.rs:58 — name() = "compact"
codelet/fspec-tui/src/views/agent/slash_commands.rs:133 — palette entry
```

No `SlashCommandAction::Compact` arm exists yet in
`app/dispatch_rpc020.rs::handle_slash_command`; the catch-all
`other => self.navigator.agent.push_line(... "[notice] /<name> not yet
implemented")` is what surfaces today. RPC-047 inserts the new arm
between the existing `Role` arm and the `other` fallback.

## StreamChunk::CompactionComplete — current handling

```
AstGrep pattern `StreamChunk::CompactionComplete { $$$ARGS }` — No matches in fspec-tui/src
```

`StreamChunk::CompactionComplete { compaction_result }` is declared in
`codelet/rpc-types/src/lib.rs:774` but no consumer exists yet inside
the Rust TUI. RPC-047 adds:

- A new arm in `dispatch_rpc045.rs::handle_stream_chunk_state_updates`
  that (a) clears `compaction_progress_for(session_id)` and
  (b) dispatches `Action::EmitSessionNotice(session_id, text)` so the
  notice lands in the originating session's scrollback regardless of
  focus.

## AgentViewStore per-session push-state accessor pattern

The `store/agent_view/isolation_state.rs` sub-module already hosts
three structurally identical accessor pairs:

```
codelet/fspec-tui/src/store/agent_view/isolation_state.rs:61 — pub fn session_status_for
codelet/fspec-tui/src/store/agent_view/isolation_state.rs:76 — pub fn isolation_state_for
codelet/fspec-tui/src/store/agent_view/isolation_state.rs:90 — pub fn debug_enabled_for
```

RPC-047 adds a fourth pair — `compaction_progress_for(session: &SessionId)`
+ `set_compaction_progress(session: SessionId, progress: CompactionProgress)`
+ `clear_compaction_progress(session: &SessionId)` — alongside the new
`pub(crate) compaction_progress_by_session: HashMap<SessionId,
CompactionProgress>` field on `AgentViewStore` in
`store/agent_view.rs`.

The new accessors live in `isolation_state.rs` so `agent_view.rs`
stays under its 300-LoC ceiling per the
`agent_view_store_stays_under_300_loc_with_history_fields` source-
shape invariant pinned by `rpc025-source-shape.feature`.

## SessionFooter widget shape + sole call-site

```
codelet/fspec-tui/src/views/agent/footer.rs:50 — pub struct SessionFooter<'a> { pub workspace: Option<&'a WorkspaceInfo>, }
codelet/fspec-tui/src/views/agent.rs:280 — SessionFooter { workspace: store.workspace() }.render(footer_area, buf)
```

RPC-047 widens the struct with `pub compaction_progress:
Option<&'a CompactionProgress>` and updates the sole call-site in
`views/agent.rs::render_with_store` to seed it from
`store.compaction_progress_for(current_session)`.

## Action::EmitSessionNotice precedent (RPC-046 idiom)

The action bus already routes per-session notice emissions through a
single arm:

```
codelet/fspec-tui/src/components/mod.rs:400          — Action::EmitSessionNotice(SessionId, String)
codelet/fspec-tui/src/app/dispatch.rs:290            — dispatch arm
codelet/fspec-tui/src/app/dispatch_rpc020.rs:68      — /clear success-path emitter
codelet/fspec-tui/src/app/dispatch_rpc046.rs:25–32   — handler that pushes the line into the originating SessionContext
```

RPC-047 reuses this exact pattern from both the `/compact` handler in
`dispatch_rpc020.rs` AND the `StreamChunk::CompactionComplete` handler
in `dispatch_rpc045.rs`. No new Action variant is required.

## FspecBackend wire-level surface

```
codelet/fspec-tui/src/transport/mod.rs:273–278  — get_compaction_progress default-impl returns None
codelet/fspec-tui/src/transport/mod.rs:295–303  — compact_session default-impl returns a 0-tokens CompactionResult
```

`EmbeddedFspecBackend` and `WebSocketFspecBackend` already override
both methods to delegate to `FspecService::*` (verified by the
cross-transport parity coverage in
`codelet/fspec-tui/tests/rpc037_cross_transport_parity.rs`). RPC-047
adds no new transport-level methods.

## Test-fixture extension surface

`codelet/fspec-tui/tests/common/mod.rs::MockBackend` needs three new
fields to drive RPC-047 scenarios:

- `compact_session_calls: AtomicUsize`
- `last_compact_session: Mutex<Option<SessionId>>`
- `compact_session_result: Mutex<Result<CompactionResult, String>>`
  (seedable via `set_compact_session_result_ok` and
  `set_compact_session_result_err`)

Plus the `FspecBackend for MockBackend` impl overrides
`compact_session` to bump the counter, capture the SessionId, and
return the scripted result. Mirrors the structure already in place
for `clear_history` (RPC-046).
