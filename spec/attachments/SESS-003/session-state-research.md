# SESS-003: authoritative session work-unit focus

Research date: 2026-10-22. Source inspection; no runtime reproduction claimed.
Overview: `docs/research/session-work-unit-focus.md`.

## Confirmed implementation gaps

- `rust/sessions/src/background_session.rs:459` stores a private optional context;
  `:600` initializes it to None. The setter at `:823` also rewrites the environment
  reminder, but uses inner.try_lock() and silently skips that rewrite if unavailable.
- `rust/agent-loop/src/agent_loop.rs:580` acquires the inner session lock for turn
  processing. Tool-time updates must not await that same lock and deadlock the loop;
  use deferred reconciliation or a safe session-level update channel.
- The setter broadcasts bridge metadata at `background_session.rs:877`, not a
  session-focus event consumed by the TUI. Existing RPC set/get operations delegate
  to it at `rust/sessions/src/handle_impl.rs:561` and `:569`.
- `rust/rpc-types/src/lib.rs:306` SessionInfo and `:338` SessionEvent lack focus data;
  `rust/core/src/persistence/manifest.rs:106` SessionManifest has no focus field.
- `rust/fspec-tui/src/app/dispatch_session_lifecycle.rs:75` startup/reconnect folds
  only session IDs. `dispatch_session_chrome.rs:24` hydrates other session settings,
  not work-unit context. An existing get_work_unit_context RPC alone is insufficient.
- `rust/agent-loop/src/session_search_handler.rs:468` derives the reported work-unit
  ID from a session name, rather than authoritative current focus.

## Important semantic boundary

`rust/sessions/src/session_tool_callbacks.rs:127` reads backend context.status for
write-stage gating, and missing context means no stage gating. Do not blindly reuse
the current optional context as display activity without defining this interaction.
Recommended: separate activity focus from authorization/historical work binding.
Isolation path restrictions must remain intact regardless of displayed focus.

## Proposed implementation boundary

Own the session-scoped API and authoritative state here. Board attach, tool admission
and successful status-command reconciliation call it; UI projection belongs to BUG-230.
Return/publish explicit Some(card) or clear plus monotonic revision and change source.
Resolve IDs/title/status against the correct session project, not arbitrary tool cwd.
Publish events and an initial snapshot on embedded and WS transports. Guard against
snapshot/event reordering, duplicates, lag and delayed local attach acknowledgments.
Persist focus without rewriting whole manifests per read-only call or overwriting
concurrent message updates; reuse the existing manifest mutation/revision protocol.
No rig-core -> sessions dependency: inject session-bound policy at construction.

Restore, deleted-card handling and permission-context separation remain discovery
questions. Old manifests should remain loadable; an absent old field is not a guessed
card from a title. If durable restore is approved, explicitly persist cleared state.
Decide whether SessionSearch's heuristic is removed or retained only as historical data.

## Regression plan

- Real session service: attach A, switch B, clear, idempotent repeats, two sessions.
- Two embedded/WS clients: changes from client one reach client two; reconnect starts
  with authoritative card or explicit clear, not an empty uninitialized projection.
- Slow old snapshot/attach acknowledgement after a newer clear cannot restore A.
- Work-unit ID validation uses caller project; cross-project IDs do not silently bind.
- Focus mutation while the agent holds inner lock still updates the next environment
  reminder without deadlock. No cross-session reminder leak (related BUG-095).
- Durable restore if approved: card/clear round trips, legacy manifests, done/deleted
  reconciliation, concurrent manifest writes and no current focus inferred from name.
- Clearing display activity cannot relax write-stage or worktree restrictions.

Existing targets to extend, not evidence of these new guarantees:

```sh
cd rust
cargo test --profile ci-test -p codelet-fspec-tui --test rpc050_cross_transport_parity
cargo test --profile ci-test -p codelet-sessions --test wt004_session_tool_callbacks
```

Use shared `rust/test-helpers` fixtures and scenario-to-test 1:1 mapping. Add focused
new integration targets for event fanout, restore and reminder contention after Gherkin.
