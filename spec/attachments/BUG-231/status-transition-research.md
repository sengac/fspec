# BUG-231: status-command switching and completed-card focus clearing

Research date: 2026-10-22. Source-level gap analysis, not a runtime reproduction.
Overview: `docs/research/session-work-unit-focus.md`. Depends on SESS-003.

## Prior acceptance criteria versus current Rust path

`spec/features/work-unit-context-service.feature:25` (TUI-059) promises that changing
a different card's status notifies the agent and updates its session context.
`spec/features/session-work-unit-ipc.feature:30` (TUI-060) also promises that attachment
and board badges move. Its direct shell CLI scenario explicitly does not reassign a
session. Both coverage sidecars reference absent TypeScript hooks/tests; a done card
and a populated coverage mapping do not prove standalone Rust parity.
Historical source is available via
`git show 54026fd7:src/commands/hooks/workUnitStatusHook.ts`: it explicitly updates
context and sends the IPC message. Its globally active-session lookup is unsuitable
for concurrent Rust agents; restore the behavior with caller identity, not that lookup.

The standalone handler at `rust/agent-loop/src/agent_loop.rs:690` builds DispatchRequest,
calls fspec-core and returns the result without setting a new session work-unit context.
The command at `rust/fspec-core/src/commands/update_work_unit_status.rs:92` has no
session identity, and persists the board at `:351`. WorkUnitsWatcher snapshots cannot
identify which session issued the command, so ownership must not be inferred there.

`rust/fspec-tui/src/store/agent_view/work_unit_state.rs:97` looks up the old bound ID
and only updates title/status. Updating B cannot change A's context to B. Backend
context can retain the old status even when the TUI correctly repaints the new one.
This matters to stage lookup at `rust/sessions/src/session_tool_callbacks.rs:127`.

## Completion is a policy extension, not a proven repaint regression

Current code has no done-specific detach/clear rule. The watcher can supply done and
BUG-180 can repaint it while retaining the same ID; that is current sticky-attachment
semantics. The user's requested active-work behavior requires an explicit focus-clear
policy. BUG-180 even intentionally preserves deleted per-session bindings, pinned by
`spec/features/bug-180-sessionheader-work-unit-status-sync.feature:48`.
Do not label all completion behavior a regression or silently rewrite historical tests.

## Recommended integration

- At the session-aware Fspec boundary, after a successful command result, refresh the
  authoritative card and apply the shared SESS-003 focus policy. Keep command business
  logic shared by CLI and tool; do not inject global session guesses into fspec-core.
- Same-card transitions refresh backend status; legacy different-card successful
  transitions retarget the invoking session. Invalid args, lifecycle rejection and
  quality-gate failures do not trigger a post-success switch.
- Once TOOL-026 supplies explicit attribution, specify conflict precedence. Recommended:
  explicit attribution wins over inference, including NONE and work on a different
  card while administratively transitioning another card. Never let both writers race.
- Completion clearing must compare current card and revision. A late done notification
  for A must not clear a newer focus B or a newer explicit activity on A.
- Sessionless CLI changes must not steal focus. External completion of the currently
  focused card, deletion and later work on an already-done card need explicit decisions.
- Keep task/turn done(), StreamChunk::Done and session Idle distinct from card status done.
  Idle alone is not proof that the work unit has finished.

## Proposed tests

1. Session on A successfully transitions B: expected switch/reminder/badges on legacy route.
2. Session on A fails a transition for B: no post-success context mutation.
3. Same-card status refresh updates backend context, environment and UI consistently.
4. Current card moves validating -> done: active focus clears without closing session.
5. Older completion A after newer focus B does not clear B; duplicate results are safe.
6. Explicit attribution A/NONE with business target B obeys approved precedence.
7. CLI/external edits do not attribute another card to the foreground session.
8. Two sessions on one card obey approved external-completion policy.

Useful scoped baselines (not executed during research):

```sh
cd rust
cargo test --profile ci-test -p codelet-fspec-tui --test bug180_wu_status_sync
cargo test --profile ci-test -p codelet-sessions --test wt004_session_tool_callbacks
```

Add a new real-filesystem codelet-agent-loop integration target exercising the actual
in-process Fspec handler plus events, not a UI-only action injection. Update old
TUI-059/TUI-060 coverage paths only when real Rust scenario coverage exists.
