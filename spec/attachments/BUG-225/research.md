# BUG-225 — Continue cache is not initially hydrated from backend state

## Summary and confidence
- Audit baseline: HEAD `7e60de8e`, 2026-10-21; read-only source audit.
- **Source-confirmed conditional defect; high confidence:** create/attach chrome refresh does not load continue state, leaving a fresh TUI cache at `(false, 10)` even when the backend correctly seeded `(true, 300)`.
- A bare toggle before a live update can send `(true, 10)` back to that backend, reducing the budget rather than disabling continue. Setter failure also leaves optimistic cache state without rollback.
- No reproduction, tests, builds, or endpoint calls were executed. Incident attribution remains a hypothesis.

## Source chain
1. `rust/sessions/src/session_creation_helper.rs:264` seeds profile continue state before first dispatch; `:279` reads the stored budget, and `:284` calls `set_continue_state`. A profile budget of 300 correctly becomes `(true, 300)`.
2. `rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:173` and `rust/fspec-tui/src/app/dispatch_attach.rs:107` both invoke `refresh_session_chrome`.
3. `rust/fspec-tui/src/app/dispatch_session_chrome.rs:24` refreshes model info, thinking, and role (`:32`, `:48`, `:57`), but does not call `get_continue_state`.
4. `rust/fspec-tui/src/store/agent_view/chrome_state.rs:71` returns `(false, 10)` when its continue map has no entry.
5. `rust/fspec-tui/src/app/dispatch_slash_continue.rs:29` derives command behavior from that cache. `rust/fspec-tui/src/app/continue_parser.rs:102` toggles cached false to true using default budget 10 at `:113`; cached true would instead toggle off while preserving budget.
6. `rust/fspec-tui/src/app/dispatch_slash_continue.rs:50` updates the cache optimistically, `:57` clears the live counter, and `:68` sends the absolute pair. On error, `:71` sends only a notice; it does not restore or reload state.
7. `rust/fspec-tui/src/app/dispatch_stream_chunks.rs:153` later hydrates both cached and live state from ContinueStateUpdate. Exposure is before a suitable update, or when attaching without a replayed update.
8. `rust/fspec-tui/src/transport/mod.rs:415` already exposes a continue getter. `rust/sessions/src/handle_impl.rs:1431` writes backend chrome fields; `:1447` provides its getter.

## Impact and lifecycle boundaries
- Idle footer/command behavior can disagree with the real backend seed. A stale bare toggle can overwrite 300 with 10, which is more than a visual defect.
- `rust/sessions/src/background_session.rs:1424` explicitly applies setter changes on the next dispatched user message, not to the active stream. `:1529` copies chrome state into the inner session; `rust/agent-loop/src/agent_loop.rs:576` invokes that sync at dispatch.
- Active-stream reconfiguration is therefore a separate CONT-010 contract question; this card must not promise immediate hot updates.
- Profile defaults are creation-time seeds, not locks or switch-time reseeds (`spec/features/profile-auto-continue-session-seeding.feature:9`). Current profile switching does **not** reseed continue state; that behavior must not be mislabeled as this hydration defect.

## Reproduction outline — proposed, not executed
1. In an isolated offline fixture, create a backend session using profile `autoContinue: 300`; verify backend getter returns `(true, 300)`.
2. Create/attach a fresh TUI store to that session, await chrome refresh tasks, and withhold live stream chunks.
3. Inspect cached state/footer, then submit bare `/continue`. The source predicts local false/10 becomes true/10 and the backend receives that absolute pair.
4. Compare with a hydrated true/300 cache, where bare toggle should disable continue and retain 300.
5. Inject setter failure and inspect cache/backend divergence; separately prove inner active-turn state remains unchanged until next dispatch.

## Existing specifications/tests and missing regressions
- `spec/features/profile-auto-continue-session-seeding.feature:27` and `rust/sessions/tests/prov142_session_seed.rs:110` cover backend seeding of 300, not TUI hydration.
- `spec/features/live-continue-status-indicator.feature:42` covers live footer updates; `:50` covers clearing stale counters after slash dispatch.
- `rust/fspec-tui/tests/cont007_live_continue_status_test.rs:333` explicitly seeds cache using `/continue 10`; `:405` injects a live update before slash dispatch. Neither tests fresh attachment to a backend-seeded session.
- `spec/features/continue-command-surface.feature:21` defines the default bare-toggle surface; `spec/features/completion-contract-dispatch-arming.feature:41` covers dispatch sync, not active-stream changes.
- Missing: create and attach hydration with backend true/300 and false/custom budget; a toggle before first stream; failed getter/setter behavior; focus changes and late fetches that must not overwrite newer commands/live updates.

## Recommended direction
- Hydrate continue state on create/attach through a session-keyed loaded action; distinguish unknown/loading from a real off/default state before applying relative toggles.
- Prevent late hydration responses from overwriting newer commands or stream updates. On setter failure, reconcile from backend or roll back only the matching optimistic operation.
- Preserve next-dispatch semantics and creation-only profile seeding unless CONT-010 explicitly changes those contracts with separate acceptance criteria.

## Uncertainties and related cards
- Live updates can close this gap; reproduction depends on ordering and cache history. This audit does not establish whether the reported incident hit that timing window.
- Related: PROV-142 (correct backend seed), CONT-002/CONT-007/CONT-009 (command, live cache, dispatch sync), CONT-010 (profile/active-turn lifecycle), BUG-226 (another optimistic-cache consistency gap).
- BUG-221, BUG-222, BUG-223, and BUG-224 are independent exhaustion/retry/Gemini paths; stale TUI continue state alone does not prove any of them occurred.
