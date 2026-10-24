# BUG-230: stale session chips, fallback leakage and board badges

Research date: 2026-10-22. Confirmed source paths; runtime tests not executed.
Overview: `docs/research/session-work-unit-focus.md`. Depends on SESS-003.

## What currently works

`rust/fspec-tui/src/app/dispatch.rs:58` invokes sync_work_unit_contexts after board
reseed. `store/agent_view/work_unit_state.rs:97` refreshes title/status by an existing
binding's ID. This is BUG-180's fix, not a general active-card transition mechanism.
Its tests are `rust/fspec-tui/tests/bug180_wu_status_sync.rs`.

## Confirmed gaps

- The bracketed board/agent/mux menu chips share
  `rust/fspec-tui/src/views/board/menu_snapshot.rs:57` chip_inputs, reading only the
  AgentViewStore per-session context map. Updating a backend field does not update it.
- `rust/fspec-tui/src/views/agent/chrome_paint.rs:48` also falls back to the global
  current_work_unit_id/status when the focused session has no context. An unbound
  session B can therefore display A's previous card; menu and header may disagree.
- `rust/fspec-tui/src/app/dispatch_work_unit_entry.rs:17` writes the legacy slots and
  queues AttachWorkUnitToSession for the current session.
- Successful attach at `dispatch_work_unit_binding.rs:75` only updates AgentViewStore.
  The separate BoardStore.session_attachments badge map is not migrated there.
- `rust/fspec-tui/src/store/board.rs:220` attach_session already removes a session's
  old card mapping, but existing-session Enter does not invoke that operation.
  New-session creation does queue both actions at `dispatch_create_session_dialog.rs:162`.
- `dispatch_session_chrome.rs:24` does not fetch focus; reconnect snapshot handling
  at `dispatch_session_lifecycle.rs:75` discards SessionInfo except the ID.
- BUG-205 removed /detach and WorkUnitDetached. The private UI context-clear helper
  still exists, but clear no longer has a general runtime event-to-UI route.

## Proposed reproduction scenarios for failing tests

1. Enter A with an existing session, then enter B: backend/header rebind to B while
   BoardStore's session badge can remain absent or mapped to A.
2. Enter A in session one, cycle to unbound session two: AgentView header can paint A
   via fallback even though session two's menu chip has no card suffix.
3. Attach a card through one daemon client, then connect a fresh client: the fresh
   local map is unpopulated despite the daemon retaining context.
4. Clear focus through SESS-003: neither a legacy fallback nor an older asynchronous
   attach reply may resurrect A; keep the session alive and navigable.

## Required changes and scope guard

Consume revisioned authoritative events/snapshots on App's single-writer dispatch path.
Update session header context and board badge projection together on switch/clear.
Distinguish unknown/not-hydrated focus from known NONE; eliminate global fallback for
known sessions rather than hiding the text in one painter. Preserve unbound-session
and pre-session rendering behavior deliberately, with explicit tests.
Do not reintroduce /detach: activity clearing is not closing or detaching an agent.
Keep existing same-card title/status refresh and terminal sanitization.
Define multiple sessions working the same card: BoardStore currently maps one card to
one SessionId, whereas per-session menu chips can naturally represent multiple agents.
This cardinality is an open product decision, not permission to silently evict an agent.

## Regression targets

```sh
cd rust
cargo test --profile ci-test -p codelet-fspec-tui --test bug180_wu_status_sync
cargo test --profile ci-test -p codelet-fspec-tui --test work_unit_binding_rpc050
cargo test --profile ci-test -p codelet-fspec-tui --test menu011_menu_bar_brackets
cargo test --profile ci-test -p codelet-fspec-tui --test rpc050_cross_transport_parity
```

Add scenario-mapped rendered TestBackend tests for board, agent and mux card changes,
NONE, status/title updates and second-client hydration. Include delayed acknowledgments,
two-session isolation, badge migration and old snapshot rejection. These commands are
recommended scoped baselines, not tests executed during this research.
