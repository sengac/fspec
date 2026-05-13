# Review: RPC-011 — DisconnectDialog CR-1 baseline + Auto-reconnect supervisor

## Status: WARN

## 🔴 Critical Issues (Must Fix)

None — no test failures, no `.unwrap()`/`todo!()` in production code, no `@step` text mismatches, no Gherkin Given/When/Then ordering issues, no placeholder text in feature files.

## 🟡 Warnings (Should Fix)

1. **Test file feature header points to a non-existent feature file** — `codelet/fspec-tui/tests/disconnect_dialog_slice1_rpc011.rs:4` and `codelet/fspec-tui/tests/auto_reconnect_slice2_rpc011.rs:3` both declare `Feature: spec/features/fspec-binary-polish-rpc011.feature`, but that file does not exist (the work was split into `disconnect-dialog-cr1-baseline.feature` and `auto-reconnect-supervisor.feature`). Per ACDD test-file-header rules the header MUST reference the actual feature file driving these scenarios. This is misleading for future readers and breaks grep-based traceability between tests and acceptance criteria.

2. **`client.rs` wires `connect_with_supervisor` to a dropped action bus (`action_tx_placeholder`)** — `codelet/fspec/src/client.rs:35-39` creates a `(tx, _rx)` pair where `_rx` is immediately dropped, then passes `tx` to `connect_with_supervisor`. Every `Action::Disconnected/Reconnecting/Reconnected/SessionCreated` the supervisor emits will hit `SendError` because no receiver is alive. The App that's constructed on line 42 has its own action bus that is NEVER hooked up to the supervisor. Rule [21] explicitly says: *"client.rs is updated to call connect_with_supervisor(url, app.action_tx_clone())"*. The TODO-style comment on lines 33-34 acknowledges this (`The App grows action_tx_clone() for exactly this purpose.`) but the implementation defers to a placeholder. **End-to-end the disconnect dialog will NEVER appear in the released binary** — tests pass only because they construct the action bus first and inject it directly via `unbounded_channel`. This violates rule [21] and rule [16]'s "additive only" intent because the feature is non-functional in production despite the trait/signature additions being in place. Note `App::action_tx_clone()` (app.rs:176) is defined for exactly this purpose but has zero call sites outside doc comments.

3. **`request_manual_reconnect()` is dead code in the production wiring** — `codelet/fspec-tui/src/transport/websocket.rs:137-139` exposes a public method to trigger `manual_reconnect.notify_one()`, and the App's `r`-key handler comment says the supervisor is "subscribed via `WebSocketFspecBackend::request_manual_reconnect`" (app.rs:407). However, `app.rs:409` only emits `Action::ManualReconnect` onto the action bus, and there is **NO `Action::ManualReconnect =>` arm in `dispatch()`** (grep confirms zero match arms anywhere in the workspace). The `manual_reconnect` `Notify` is therefore unreachable in production — pressing `r` swallows the key but the supervisor's `tokio::select!` arm on `manual_reconnect.notified()` (websocket.rs:308) never fires. Scenario 5 ("Pressing r ... resets backoff") passes only because it asserts the bus emission, not the supervisor-side cancel. Rule [4] / example [4] / Gherkin step "And on the next failure the backoff schedule restarts from 250ms" is NOT actually wired end-to-end. The test on line 360-365 of `disconnect_dialog_slice1_rpc011.rs` explicitly defers this assertion to slice 2, but slice 2 also does not verify it.

4. **`WebSocketFspecBackend.action_tx` field is stored but never read** — `codelet/fspec-tui/src/transport/websocket.rs:52` declares `action_tx: Option<UnboundedSender<Action>>`. It's populated at lines 70 (`None`) and 131 (`Some(action_tx)`). A workspace-wide grep for `self.action_tx` against this file returns zero hits. The supervisor task receives its own clone via `supervisor_action_tx` (line 115). The field is dead code and should be removed, or wired to a real consumer (e.g. as the read-side for routing `Action::ManualReconnect` into `manual_reconnect.notify_one()` — which would resolve Warning #3).

5. **`map_err(|e| anyhow::Error::from(e))` redundancy** — `codelet/fspec-tui/src/transport/websocket.rs:149,158,167,179,192,237` (six occurrences) all use `.ok_or(BackendError::Disconnected).map_err(|e| anyhow::Error::from(e))?`. The closure is a thin wrapper around `From::from` and triggers `clippy::redundant_closure`. The idiomatic form is `.ok_or(BackendError::Disconnected)?` (since `BackendError: Error + Send + Sync + 'static` and `anyhow::Result<T>` already accepts via `?`'s built-in `From`) or `.map_err(anyhow::Error::from)?`.

6. **Over-broad / mis-targeted impl line ranges in coverage links**:
   - `disconnect-dialog-cr1-baseline.feature.coverage` scenarios 2, 3, 4 each link impl lines **1..231** (the WHOLE of `disconnect_dialog.rs`, which is only 231 lines — i.e. literally every line including the `#[cfg(test)]` module at lines 174-231). For scenario 3 (key-swallowing) the relevant code is `handle_event` at lines ~109-143; for scenario 4 (`q` exits client) the relevant code is in `app.rs:392-417` AND `disconnect_dialog.rs:112-121` — but `app.rs` isn't linked at all and `disconnect_dialog.rs` is linked whole-file.
   - `auto-reconnect-supervisor.feature.coverage` scenario 3 ("Reconnect re-issues create_session(None)") links `websocket.rs:292-361` when the actual code under test is `app.rs:345-368` (the `Action::Reconnected` arm in `dispatch`). The supervisor in websocket.rs does NOT call `create_session` — `App.dispatch` does (architecture note [3] / rule [23]).
   - Scenario 4 ("Reconnecting Action updates the dialog text inline") links `websocket.rs:292-340` — wrong; the code under test is `disconnect_dialog.rs:149-154` (the `update()` method receiving `Action::Reconnecting(n)`) plus the rendering at lines 156-171.
   - Scenario 5 ("ServerGoingAway") only links `rpc-server/src/server.rs:63-70` (the `request_shutdown` fn) but the test also exercises the client-side supervisor's drop-detect path at `websocket.rs:267-289`.

7. **Coverage gap: App-side disconnect/reconnect dispatch logic not linked from either feature** — The `Action::Disconnected` (app.rs:335-344) and `Action::Reconnected` (app.rs:345-368) handlers in `app.rs` are the production code paths that make scenarios 2 + 5 (slice 1) and 3 + 4 (slice 2) pass, but **no coverage entry references `app.rs` for any RPC-011 scenario.** The `Disconnected` arm pushes the dialog; the `Reconnected` arm pops it and re-issues bootstrap. Without these arms wired, the supervisor's action emissions would be no-ops.

8. **Two Gherkin steps in scenario 4 (slice 1) have `@step` comments but no real assertions** — `codelet/fspec-tui/tests/disconnect_dialog_slice1_rpc011.rs:286-292`:
   - `// @step And App::run returns Ok(()) and the client process exits with status 0` — followed by a comment saying *"we don't drive the full run loop here to keep this scenario hermetic"*.
   - `// @step And no panic backtrace is printed on stderr` — followed by *"Assertion is implicit: this test would panic if any layer paniced."*

   These are essentially documentation-only `@step` comments. They satisfy link-coverage's `@step` requirement but don't verify behaviour. Either run the actual `App::run()` (slow but real) or move these steps into a dedicated end-to-end test, or downgrade the scenario by removing the unverifiable steps from the feature file.

## 🟢 Observations (Nice to Have)

1. **`@step` comment fidelity is excellent.** All 27 Gherkin steps in `disconnect-dialog-cr1-baseline.feature` map to byte-exact `// @step` comments in `disconnect_dialog_slice1_rpc011.rs`, and all 24 steps in `auto-reconnect-supervisor.feature` map to byte-exact comments in `auto_reconnect_slice2_rpc011.rs`. No drift detected — including the em-dash and ellipsis in `"auto-reconnecting (attempt 1)…"`.

2. **No `.unwrap()` / `panic!` / `todo!()` / `unimplemented!()` in production source.** Both `transport/websocket.rs` and `components/disconnect_dialog.rs` have zero violations. The two `.expect(...)` calls in `disconnect_dialog.rs` (lines 185, 190) are inside `#[cfg(test)] mod tests` — correct.

3. **File sizes are healthy.** websocket.rs = 362 LOC, disconnect_dialog.rs = 231 LOC, test files 366/345 LOC. websocket.rs is slightly over the 300-line guidance but the supervisor is a cohesive single concern and splitting would scatter related state.

4. **Both feature files have proper `@RPC-011` tag, an architecture doc string after the `Feature:` line, and a structured `Background: User Story` matching the work-unit narrative.** No prefill placeholders detected.

5. **Backoff schedule constant is well-named and matches scenario data table exactly** — `BACKOFF_SCHEDULE_MS: &[u64] = &[250, 500, 1_000, 2_000, 5_000]` (websocket.rs:29) lines up 1:1 with the Gherkin data table in `auto-reconnect-supervisor.feature:33-40`. Backoff index calculation `(attempt - 1).min(last)` correctly caps at 5000ms for attempts 5+.

6. **Scenario 1's "internal client slot becomes None" is verified indirectly** via `health().await.unwrap_or(true)` rather than a direct accessor. Consider exposing `pub fn is_connected(&self) -> bool` on `WebSocketFspecBackend` so the assertion is precise — currently the assertion could spuriously pass on any non-Disconnect error.

7. **`pump_actions` helper is duplicated verbatim** between `disconnect_dialog_slice1_rpc011.rs:47-51` and `auto_reconnect_slice2_rpc011.rs:31-35`; same for `render_to_string` at lines 57-70 / 38-51. Both could live in `tests/common/mod.rs`.

8. **`let _: &dyn Component = &DisconnectDialog::new();` (slice 2, line 297)** is a no-op "belt-and-braces" assertion that proves only Component-ness, which is already proven by `Compositor::push(Box::new(DisconnectDialog::new()))` elsewhere. The `use Component;` import (line 22) exists solely for this line — drop both.

9. **`empty_broadcast_rx()` helper (websocket.rs:246-250)** is a clever way to give callers a pre-closed receiver, but the call sites in `chunks_rx/work_units_rx/logs_rx` (lines 197-230) will return such a closed receiver while the supervisor is rebuilding the inner client. This means the App's subscriber tasks observe `RecvError::Closed` and exit during every reconnect cycle — the `Reconnected` dispatch arm in app.rs:345-368 does NOT respawn them (only re-issues `list_work_units` + `create_session`). So broadcast subscribers die on first disconnect and stay dead, which silently breaks live chunk streaming after the first reconnect. Not in scope for the feature files reviewed but worth flagging — likely already covered by a sibling RPC-011 work item or worth a follow-up card.

10. **Architecture rule [5] / scenario "Auto-reconnect happy path"** says: *"it respawns the three subscriber tasks against the new chunks/logs/work_units broadcasts"*. The supervisor's `run_supervisor` does NOT respawn these subscriber tasks — it only swaps the `FspecWsClient` in the slot. The App's `Action::Reconnected` arm also does not respawn them. The test asserts only that `Action::Reconnected` was emitted, not that the subscribers are re-attached. This is the same root cause as observation #9.

## Coverage Verification
- Feature file: `spec/features/disconnect-dialog-cr1-baseline.feature` — OK (5 scenarios, proper @RPC-011 tag, no placeholders, doc-string architecture present)
- Feature file: `spec/features/auto-reconnect-supervisor.feature` — OK (5 scenarios, proper @RPC-011 tag, no placeholders, doc-string architecture present)
- Test file(s): `codelet/fspec-tui/tests/disconnect_dialog_slice1_rpc011.rs`, `codelet/fspec-tui/tests/auto_reconnect_slice2_rpc011.rs` — ISSUE: header points to non-existent `fspec-binary-polish-rpc011.feature` (Warning #1); two steps in slice-1 scenario 4 have `@step` markers without real assertions (Warning #8)
- Impl file(s): `codelet/fspec-tui/src/transport/websocket.rs`, `codelet/fspec-tui/src/components/disconnect_dialog.rs`, `codelet/fspec-tui/src/components/mod.rs`, `codelet/fspec-tui/src/app.rs` — ISSUE: `app.rs` dispatch arms (lines 335-368, 392-417) NOT linked from coverage (Warning #7); `client.rs:35-41` wires a dropped action bus making the feature non-functional in production (Warning #2); `Action::ManualReconnect` has no dispatch arm so `manual_reconnect` Notify is unreachable (Warning #3); dead `action_tx` field on backend (Warning #4)
- Scenario coverage: 10/10 scenarios have test mappings; tests pass 5/5 + 5/5; however 4 scenarios have impl line ranges that don't point at the actual code under test (Warning #6)

## Files Reviewed
- `/Users/rquast/projects/fspec/spec/features/disconnect-dialog-cr1-baseline.feature`
- `/Users/rquast/projects/fspec/spec/features/auto-reconnect-supervisor.feature`
- `/Users/rquast/projects/fspec/spec/features/disconnect-dialog-cr1-baseline.feature.coverage`
- `/Users/rquast/projects/fspec/spec/features/auto-reconnect-supervisor.feature.coverage`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/tests/disconnect_dialog_slice1_rpc011.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/tests/auto_reconnect_slice2_rpc011.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/transport/websocket.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/transport/mod.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/components/disconnect_dialog.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/components/mod.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/app.rs`
- `/Users/rquast/projects/fspec/codelet/fspec/src/client.rs`
- Work unit RPC-011 via `fspec show-work-unit` (rules [0]-[5], [16], [21]-[23], architecture notes [0]-[3], examples [0]-[5])

---

**Summary**: Code quality, Gherkin syntax, `@step` fidelity, and ACDD test discipline are strong. However, the **production wiring in `client.rs` is broken** — the supervisor's action emissions go to a dropped channel, and `Action::ManualReconnect` has no dispatch handler — meaning the disconnect dialog and `r`-press reconnect features will not work end-to-end in the released `fspec client` binary despite all 10 tests passing. This is a "tests prove the units, but the integration is unwired" pattern. The tests themselves are correctly constructed but inject the action bus directly, masking the production gap.
---
# Review: RPC-011 — daemon-lifecycle-signals + stale-daemon-json-autodiscovery

## Status: WARN

## 🔴 Critical Issues (Must Fix)

1. **Gherkin claims watcher rebuild behavior that does NOT happen in code.** `spec/features/daemon-lifecycle-signals.feature:41-44` says SIGHUP "constructs a fresh WorkUnitsWatcher W_new", "service.watcher.store(Arc::new(W_new)) replaces the old watcher atomically", and "subsequent list_work_units calls observe snapshots from W_new". But `codelet/fspec/src/daemon.rs:70-81` explicitly comments: "full ArcSwap migration is left as a follow-up; this baseline at least logs SIGHUP and keeps the daemon alive". The watcher is NOT rebuilt. The test (`codelet/fspec/tests/daemon_lifecycle_rpc011.rs:121-161`) only asserts the daemon is still alive 500ms after SIGHUP — it cannot detect the divergence. This is a silent acceptance-criteria failure: either downgrade the Gherkin to match reality ("logs SIGHUP and continues; watcher rebuild deferred") or complete the ArcSwap implementation.

2. **Both test file headers reference a non-existent feature file.** `codelet/fspec/tests/daemon_lifecycle_rpc011.rs:3` and `codelet/fspec/tests/stale_daemon_json_rpc011.rs:3` both read `//! Feature: spec/features/fspec-binary-polish-rpc011.feature`. That file does not exist (only `rpc011-regression-invariants.feature` does). Headers must reference `spec/features/daemon-lifecycle-signals.feature` and `spec/features/stale-daemon-json-autodiscovery.feature` respectively. This violates the test-file-header convention from CLAUDE.md and breaks coverage traceability.

3. **Panic-path scenario step is unverified.** `daemon-lifecycle-signals.feature:74-76` ("When the process panics inside handle_connection / Then … remove_daemon_json runs before the default backtrace fires") is marked covered by a code comment at `daemon_lifecycle_rpc011.rs:446-455` that points to "source_shape_rpc011.rs" — but ripgrep finds NO such file anywhere in the tree (only the comment referencing it). Furthermore, `install_panic_hook()` at `codelet/fspec/src/common.rs:122-136` does NOT call `remove_daemon_json` — the Gherkin claim is currently false. Add real coverage (debug-only panic injection knob OR extend the panic hook AND add a source-shape test) or downgrade the Gherkin step.

## 🟡 Warnings (Should Fix)

1. **`@step` text mismatch with implementation symbol.** `daemon_lifecycle_rpc011.rs:58` carries `// @step Then common::request_shutdown_via_stats fires …`, mirroring `daemon-lifecycle-signals.feature:29`. The actual function is `codelet_rpc_server::request_shutdown` (defined `codelet/rpc-server/src/server.rs:63`) — no symbol named `request_shutdown_via_stats` exists. Rename in both the Gherkin step and the matching @step comment.

2. **`read_daemon_json_port` is dead code.** `codelet/fspec/src/common.rs:374-377` defines `pub fn read_daemon_json_port(path: &Path) -> Result<u16>` but no caller in the entire workspace references it (the only ripgrep hit is its own definition). RPC-011 rule [20] promised it would stay "as a legacy accessor that delegates to read_and_verify_daemon_json" — but client.rs and status.rs both bypass it and call `read_and_verify_daemon_json` directly. Either remove the function or document its public-API rationale explicitly.

3. **Stale-daemon stderr assertion is too lax.** `stale_daemon_json_rpc011.rs:78-83` accepts EITHER `"no daemon.json found"` OR `"stale daemon.json removed"`. `stale-daemon-json-autodiscovery.feature:30` mandates the stable text `"no daemon.json found"`. The OR-branch undermines the contract that callers can match on. Tighten to require `"no daemon.json found"` (AND optionally `"stale"`).

4. **Obsolete-import smell in stale test.** `stale_daemon_json_rpc011.rs:200-202` contains `fn _silence_unused() { let _ = spawn_fspec_daemon; }` because line 23 still imports `spawn_fspec_daemon` even though no test uses it. Drop the import and the placeholder fn.

5. **Drain-timing literal disagrees with Gherkin.** `daemon-lifecycle-signals.feature:32` reads "awaits the bind_and_serve JoinHandle for up to 5 seconds before aborting it". `daemon.rs:97-100` actually does `sleep(500ms) → join.abort() → timeout(1s, join)` — no 5s grace. Either update the Gherkin to "for up to 500ms" or implement the 5s wait that was specified.

6. **ConnectedClientGuard test observability is fudged.** `daemon_lifecycle_rpc011.rs:240-243` and `:281` accept `"connected_clients: 0"` OR `"1"` (precondition) and `"1"` OR `"2"` (post-upgrade) to paper over the fact that `fspec status` itself opens a one-shot WS that perturbs the counter. The Gherkin step says "reads 1 immediately after the upgrade succeeds". Consider a direct in-process assertion via an embedded backend / `ServerStatsHandle` accessor (architecture note [3] mentions exposing one) so the test actually measures what the Gherkin claims.

7. **Blanket `clippy::panic` allow in tests is too broad.** Both `daemon_lifecycle_rpc011.rs:18` and `stale_daemon_json_rpc011.rs:15` carry `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]`. `unwrap_used`/`expect_used` are reasonable in tests, but the `panic` allow is broad — prefer descriptive `.expect("…")` everywhere instead.

## 🟢 Observations (Nice to Have)

1. `codelet/fspec/src/common.rs` is 454 lines — well over the 300-line guidance in CLAUDE.md. Consider splitting daemon.json concerns (`DaemonHandshake`, `daemon_json_path`, `write_daemon_json`, `read_and_verify_daemon_json`, `pid_is_alive`, ISO 8601 helpers) into a `daemon_json.rs` submodule.

2. The in-house ISO 8601 formatter (`system_time_iso8601` + `civil_from_days`, `common.rs:211-242`) is correct but worth noting — a small `time` or `humantime` dep would be more maintainable than carrying Howard Hinnant's days-from-civil inverse by hand. Not a blocker.

3. `daemon.rs:100` swallows the `tokio::time::timeout(Duration::from_secs(1), join).await` result silently. Tracing whether the abort completed vs timed out would aid future debugging of slow drains.

4. The SIGHUP-coverage line range from `fspec show-coverage` maps to `daemon.rs:65-83` — that range includes the SIGINT/SIGTERM match-arm too. Tightening to lines 70-81 (the `Sighup` arm only) would be more accurate.

5. ZERO `.unwrap()`, `todo!()`, or `unimplemented!()` in production files (`common.rs`, `daemon.rs`, `client.rs`, `server.rs`). ✅

6. No `println!`/`eprintln!`/floating promises in the four production files reviewed; `Result` plumbing looks consistent with anyhow + `?` discipline elsewhere in the workspace.

## Coverage Verification

- Feature file: `spec/features/daemon-lifecycle-signals.feature` — OK (6 scenarios, well-formed Gherkin, `@RPC-011` tag present, doc-string architecture block present)
- Feature file: `spec/features/stale-daemon-json-autodiscovery.feature` — OK (3 scenarios, `@RPC-011` tag present)
- Test file: `codelet/fspec/tests/daemon_lifecycle_rpc011.rs` — ISSUE: header references non-existent `fspec-binary-polish-rpc011.feature` (Critical #2); SIGHUP test cannot detect missing watcher rebuild (Critical #1); panic-path step covered only by a code-review comment pointing to a file that doesn't exist (Critical #3)
- Test file: `codelet/fspec/tests/stale_daemon_json_rpc011.rs` — ISSUE: same stale header (Critical #2); `_silence_unused` placeholder (Warning #4); stale-text assertion too lax (Warning #3)
- Impl files: `codelet/fspec/src/daemon.rs` (108 LOC ✅), `codelet/fspec/src/common.rs` (454 LOC ⚠ over 300), `codelet/fspec/src/client.rs` (132 LOC ✅), `codelet/rpc-server/src/server.rs` (303 LOC ✅ at the boundary)
- Scenario coverage: daemon-lifecycle 6/6 reported by `fspec show-coverage`, BUT scenarios 2 (SIGHUP) and 6 (panic path) are observably miscovered — see Critical #1 and #3
- Scenario coverage: stale-daemon-json 3/3 reported and genuinely covered, modulo Warning #3

## Files Reviewed

- `/Users/rquast/projects/fspec/spec/features/daemon-lifecycle-signals.feature`
- `/Users/rquast/projects/fspec/spec/features/stale-daemon-json-autodiscovery.feature`
- `/Users/rquast/projects/fspec/codelet/fspec/tests/daemon_lifecycle_rpc011.rs`
- `/Users/rquast/projects/fspec/codelet/fspec/tests/stale_daemon_json_rpc011.rs`
- `/Users/rquast/projects/fspec/codelet/fspec/tests/common/mod.rs`
- `/Users/rquast/projects/fspec/codelet/fspec/src/daemon.rs`
- `/Users/rquast/projects/fspec/codelet/fspec/src/common.rs`
- `/Users/rquast/projects/fspec/codelet/fspec/src/client.rs`
- `/Users/rquast/projects/fspec/codelet/rpc-server/src/server.rs`
- `/Users/rquast/projects/fspec/codelet/rpc-server/src/lib.rs` (ServerStats fields, lines 60-200)
- `/Users/rquast/projects/fspec/codelet/rpc-server/src/pump.rs` (shutdown_signal handling, lines 155-240)
- `fspec show-work-unit RPC-011` (rules + linked features)
- `fspec show-coverage daemon-lifecycle-signals`
- `fspec show-coverage stale-daemon-json-autodiscovery`

---
# Review: RPC-011 — health() RPC + ServerStats extensions + `fspec status` subcommand

## Status: WARN

## 🔴 Critical Issues (Must Fix)

None — both test suites pass (5/5 health_rpc, 4/4 status), coverage links exist at 100%, no `.unwrap()` / `todo!()` / `unimplemented!()` / `panic!` in production code paths.

## 🟡 Warnings (Should Fix)

1. **@step text mismatch in `health_rpc_serverstats_rpc011.rs`** — Gherkin says `Given a daemon with…` but the test prepends "fspec":
   - `codelet/rpc-server/tests/health_rpc_serverstats_rpc011.rs:166` says `// @step Given a fspec daemon with the chunks broadcast capacity set to 1024`, feature `spec/features/health-rpc-serverstats.feature:45` says `Given a daemon with the chunks broadcast capacity set to 1024`.
   - `codelet/rpc-server/tests/health_rpc_serverstats_rpc011.rs:215` says `// @step Given a fspec daemon with an empty workspace`, feature `spec/features/health-rpc-serverstats.feature:54` says `Given a daemon with an empty workspace`.
   - Fix: drop "fspec" from those two `@step` lines (or update the feature — they must be identical for strict @step coverage).

2. **@step text mismatch in `status_subcommand_rpc011.rs:107`** — feature `fspec-status-subcommand.feature:33` says
   `And stdout contains the human-readable lines "fspec daemon: alive", "uptime: 14m 32s", "connected_clients: 2", "last_watcher_event: 3s ago", "broadcast_lag: chunks=0 logs=0 work_units=0"`
   but the test `@step` paraphrases values to `"uptime: ..."`, `"connected_clients: ..."`, `"last_watcher_event: ..."`. The intent is fine (the test can't reproduce 14m 32s) but @step text must match verbatim. Fix: restore the exact Gherkin text in the comment; the existing `assert!(stdout.contains("uptime:"))` shape assertions can stay.

3. **Spec/impl divergence on EmbeddedFspecBackend.health()** — `spec/features/health-rpc-serverstats.feature:41` states `EmbeddedFspecBackend implements health by reading ServerStats directly (no RPC round-trip)`, and rule [12] in the work unit repeats this. The actual implementation at `codelet/fspec-tui/src/transport/embedded.rs:90-95` routes through tarpc: `Ok(self.client.health(context::current()).await?)`. The test at `health_rpc_serverstats_rpc011.rs:142-145` only asserts the file contains the substring `"fn health"`, which passes either way. Either:
   - (a) change `embedded.rs::health()` to short-circuit via `ServerStats` directly (per Gherkin and rule [12]), or
   - (b) update the Gherkin step + rule [12] to reflect the actual "both transports share `FspecServiceImpl`" design (consistent with RPC-005's "service impl written ONCE" rule, and arguably cleaner).
   Either is acceptable; today they disagree and the test does not catch it.

4. **Trivial / structural assertions hide weak coverage** — `codelet/rpc-server/tests/health_rpc_serverstats_rpc011.rs:58-67`. The `Then it receives a HealthInfo struct over the wire` and `And HealthInfo fields are: …` steps are covered only by compile-time type-ascription locals (`let _uptime: u64 = health.uptime_secs;`). The runtime assertion is only `assert!(!version.is_empty())`. The step `And the version field equals env!("CARGO_PKG_VERSION") of the daemon process` is **not** verified — non-empty ≠ equal. Fix: replace with `assert_eq!(version, env!("CARGO_PKG_VERSION"))`.

5. **Over-claimed @step coverage in lag-counter test** — `codelet/rpc-server/tests/health_rpc_serverstats_rpc011.rs:195-206`. The `@step` comments include three behaviors that are *not* runtime-verified:
   - `Then the slow subscriber's recv() yields RecvError::Lagged(1)` — `_chunks_rx_slow` is created with a leading underscore and `recv()` is never called on it.
   - `And a tracing::warn record is emitted with target="codelet_rpc_server::server" and field skipped>=1`
   - `And that warning rides the logs broadcast as a LogRecord visible to OTHER (non-lagging) clients`
   Only `stats.lag_chunks() >= 1` is actually asserted. The inline comment acknowledges the behaviors are "already-wired" by earlier work — but the @step contract says the test verifies them. Fix: either pare the @step comments back to what's verified, or add an actual `recv()` on the slow channel + a second WS client subscribed to `logs_rx` that observes the `LogRecord`.

## 🟢 Observations (Nice to Have)

1. **`format_uptime` in `codelet/fspec/src/status.rs:90-101`** has three branches (`h m s` / `m s` / `s`) but only the `14m 32s` shape is exercised end-to-end. A focused unit test for boundaries (0s, 59s, 60s, 3599s, 3600s) would harden the format contract.

2. **`version:` line not asserted in `fspec status` test** — `status.rs:87` prints `version: <X>` but `status_subcommand_rpc011.rs:108-141` doesn't assert `stdout.contains("version:")`. Implicitly covered by the "same health() RPC and output sequence applies" step in scenario 4; an explicit assertion would be tighter.

3. **`SharedFspecService::stats` uses `AsyncMutex<Option<...>>` for what is effectively a write-once value** — `codelet/rpc/src/lib.rs:125`. `OnceCell` or `ArcSwapOption` would express "set once, read many" more clearly and remove the `.await` from the read path in `health()`. Not a bug; current code is correct.

4. **File-content-scan tests are brittle** — `health_rpc_serverstats_rpc011.rs:74-152` reads source text and `body.contains(...)` against substrings. These work but break on formatting/refactors. Consider `AstGrep` or compile-time trait-presence checks for harder coverage.

5. **`empty_broadcast_rx` helper** (`codelet/fspec-tui/src/transport/websocket.rs:246-250`) is a clean trick for the disconnected slot case; worth a brief docstring example for future readers (already commented inline).

6. **All reviewed files are well under the 300 LOC guideline** (status.rs 101, server.rs 303, websocket.rs 362 — slightly over, but tight to its single responsibility).

7. **Architecture doc-strings on both feature files are concise and accurate.** G/W/T ordering correct, no placeholders, `@RPC-011` work-unit tag present on both.

8. **`Mode::Status { connect }` clap wiring** in `codelet/fspec/src/main.rs:79-83, 95` is symmetric with `Daemon`/`Client` and well-documented.

## Coverage Verification

- Feature file: `spec/features/health-rpc-serverstats.feature` — OK (5 scenarios, `@RPC-011` tag, architecture doc string, valid Gherkin).
- Feature file: `spec/features/fspec-status-subcommand.feature` — OK (4 scenarios, `@RPC-011` tag, architecture doc string, valid Gherkin).
- Test file: `codelet/rpc-server/tests/health_rpc_serverstats_rpc011.rs` — ISSUE: 2 @step text drifts (warning #1); 1 step unverified (warning #4); 3 steps over-claimed (warning #5); type-ascription-only assertions (warning #4).
- Test file: `codelet/fspec/tests/status_subcommand_rpc011.rs` — ISSUE: 1 @step text drift (warning #2). Otherwise solid integration tests using real spawned `fspec` binary + isolated `XDG_RUNTIME_DIR`.
- Impl file: `codelet/rpc-types/src/lib.rs` — OK (`HealthInfo` is `#[cfg_attr(feature="napi", napi_derive::napi(object))]` + `Serialize + Deserialize + Clone + Debug + PartialEq + Eq`; sits alongside `WorkUnitInfo` / `SessionInfo`; well-documented).
- Impl file: `codelet/rpc/src/lib.rs` — OK (`health()` declared on `FspecService` trait, implemented on `FspecServiceImpl`; `DEFAULT_CHUNKS_CAPACITY=1024`, `DEFAULT_LOGS_CAPACITY=4096`, `DEFAULT_WORK_UNITS_CAPACITY=256`; `ServerStatsHandle` trait abstracts the read side cleanly).
- Impl file: `codelet/rpc-server/src/lib.rs` — OK (`ServerStats` carries `connected_clients`, `last_watcher_event_at`, `lag_chunks/logs/work_units`; `handle()` adapter via `ServerStatsRead`).
- Impl file: `codelet/rpc-server/src/server.rs` — OK (lag counters wired into all three fanout tasks: lines 241-243, 268-270, 295-297; `last_watcher_event_at` stamped at initial-snapshot send line 149-151 AND on each Ok snapshot line 231-233).
- Impl file: `codelet/fspec-tui/src/transport/mod.rs` — OK (trait method `async fn health(&self) -> Result<HealthInfo>` at line 86).
- Impl file: `codelet/fspec-tui/src/transport/embedded.rs:90-95` — **DISCREPANCY** with Gherkin step at `health-rpc-serverstats.feature:41` ("no RPC round-trip"); implementation uses tarpc (warning #3).
- Impl file: `codelet/fspec-tui/src/transport/websocket.rs:232-239` — OK (delegates through `client.client().health(context::current()).await` exactly as the Gherkin step at line 42 requires).
- Impl file: `codelet/fspec/src/status.rs` — OK (101 LOC; clean error paths via `eprintln!` + `std::process::exit(1)`; `resolve_url` distinguishes "no daemon.json" vs "stale daemon.json removed" via stable error text).
- Impl file: `codelet/fspec/src/main.rs:79-95` — OK (`Mode::Status { connect }` correctly wired through to `status::run(connect)`).
- Scenario coverage: **9 / 9 scenarios linked** (5 health-rpc + 4 status); both `.feature.coverage` files report 100%; impl line-range mappings present for every scenario.

## Files Reviewed

- `spec/features/health-rpc-serverstats.feature`
- `spec/features/fspec-status-subcommand.feature`
- `spec/features/health-rpc-serverstats.feature.coverage`
- `spec/features/fspec-status-subcommand.feature.coverage`
- `codelet/rpc-server/tests/health_rpc_serverstats_rpc011.rs`
- `codelet/fspec/tests/status_subcommand_rpc011.rs`
- `codelet/rpc-types/src/lib.rs`
- `codelet/rpc/src/lib.rs`
- `codelet/rpc-server/src/server.rs`
- `codelet/rpc-server/src/lib.rs`
- `codelet/fspec-tui/src/transport/mod.rs`
- `codelet/fspec-tui/src/transport/embedded.rs`
- `codelet/fspec-tui/src/transport/websocket.rs`
- `codelet/fspec/src/status.rs`
- `codelet/fspec/src/main.rs`
- `fspec show-work-unit RPC-011` (rules [9], [10], [11], [12], [13], [24])
# Review: RPC-011 — Multi-Client + Tracing & Regression Invariants

## Status: WARN

## 🔴 Critical Issues (Must Fix)

1. **Stale feature-file reference in test headers.** Both test files declare `//! Feature: spec/features/fspec-binary-polish-rpc011.feature` (multi_client_and_tracing_rpc011.rs:3 and regression_invariants_rpc011.rs:3), but no such feature file exists. The actual features are `spec/features/multi-client-and-tracing.feature` and `spec/features/rpc011-regression-invariants.feature`. Per the project standard "Test file header references the feature file," these headers MUST be corrected so coverage audits and human readers can navigate from test back to spec.

2. **@step text mismatch — "a fspec daemon" vs Gherkin "a daemon".** Gherkin steps say `Given a daemon listening on 127.0.0.1:0` (multi-client-and-tracing.feature:25) and `Given a daemon with two simultaneous clients on peer addrs …` (line 42), but the test uses `// @step Given a fspec daemon listening on 127.0.0.1:0` (multi_client_and_tracing_rpc011.rs:38) and `// @step Given a fspec daemon with two simultaneous clients on peer addrs …` (line 275). The word `fspec` is extra in the test. The fspec workflow requires @step comments to match step text exactly — link-coverage should flag this. Either drop the "fspec" word from the @step comments, or amend the Gherkin to say "Given a fspec daemon …" consistently (the latter is more descriptive and recommended).

## 🟡 Warnings (Should Fix)

1. **Trivial/surrogate assertion in the tracing scenario.** The Gherkin (multi-client-and-tracing.feature:44-45) commits to "at least two records with field client_id=127.0.0.1:54321" and the same for `:54322` — pinned ports. The test (multi_client_and_tracing_rpc011.rs:307-317) explicitly disclaims this ("We cannot pin specific ephemeral ports") and instead asserts `client_id_count >= 2`, which is satisfied by any two records bearing any `client_id=` field, possibly from the SAME connection. Either rewrite the Gherkin to describe the actual invariant ("two simultaneous connections produce records tagged with two distinct client_id values"), or bind two clients to known ports (pre-opened TcpListener pair) so the literal-port assertion is real.

2. **The "ONLY records from that connection's task" isolation invariant is not actually asserted.** Gherkin step (multi-client-and-tracing.feature:46) requires that grepping by `client_id` yields ONLY records originating from that connection's task. The test surrogate (multi_client_and_tracing_rpc011.rs:319-334) merely counts ≥2 distinct `client_id` values — it does NOT verify isolation. A bug where every record carried BOTH clients' ids would still pass. Add a correlation check: pair `client_id` with a per-connection RPC payload (e.g. the session_id returned from each client's create_session) and assert no cross-leakage.

3. **Tautological filter expression.** multi_client_and_tracing_rpc011.rs:310-313 reads `.filter(|r| r.contains("client_id=") || r.contains("client_id="))` — the same condition twice. Clearly a copy-paste leftover. Either delete the duplicate, or replace one half with the intended companion check (e.g. the span name `ws_connection`).

4. **`connect_with_supervisor` additivity check is too weak.** regression_invariants_rpc011.rs:96-99 asserts `body.contains("action_tx")`. ANY local variable, comment, or unrelated field named `action_tx` satisfies this — it does not prove `connect_with_supervisor` actually takes an `action_tx` parameter. Tighten to a substring such as `body.contains("action_tx: UnboundedSender<Action>")` or assert the full parameter line.

5. **`bind_and_serve` parameter-count check is too permissive.** regression_invariants_rpc011.rs:64-67 uses `comma_count <= 1`, so signatures with zero or one comma both pass. Use `== 1` to assert exactly two parameters; otherwise a regression that collapsed both params into one would not be caught.

6. **127.0.0.1 invariant assertion bypasses comment-strip.** regression_invariants_rpc011.rs:171-175 uses `strip_comments(&server_body).contains("\"127.0.0.1:0\"") || server_body.contains("127.0.0.1")`. The second disjunct reads the un-stripped body, so a doc-comment mentioning `127.0.0.1` would satisfy the invariant even if the binary moved to `0.0.0.0` in code. Either drop the second disjunct or apply `strip_comments` to it too.

7. **"Envelope is the sole wire-format type" is not actually asserted.** Architecture invariants scenario (rpc011-regression-invariants.feature:39) commits to "Envelope is the sole wire-format type in codelet/rpc-server/src/envelope.rs", but regression_invariants_rpc011.rs:143-147 only checks the file exists. Add a check that no other `pub enum` / `pub struct` named like a wire-format (e.g. `*Envelope`, `*Frame`) lives elsewhere in `rpc-server/src/`.

8. **Step ordering deviation in Scenario 1.** Gherkin order is `Given → When (two backends) → And (create_session) → And (send_input) → Then (same sequence) → And (connected_clients == 2) → And (no chunk lost)`. The test executes the `connected_clients == 2` assertion (multi_client_and_tracing_rpc011.rs:66-71) BEFORE `create_session`/`send_input` (lines 73-84), so an And labelled as a Then-side assertion sits in the When sequence. Not a hard Then-before-When violation, but the @step labelling is misleading. Reorder the test (do the check after the chunk-equality assertion) or reorder the Gherkin to match the actual flow.

9. **`earlier_rpc_005_010_test_suites_still_pass` is a file-existence sweep, not a "tests still pass" check.** regression_invariants_rpc011.rs:213-273 only asserts the test files exist on disk and the workspace `#[ignore]` count is ≤ 10. The Gherkin step "Then all prior tests pass" (rpc011-regression-invariants.feature:46) is enforced ONLY by CI, not by this test. Either rename the Gherkin step to "the prior test files still exist and have not been silenced," or invoke `cargo test --no-run -p <crate>` from the test to at least prove the suites still compile.

10. **Coverage line-range for "Broadcast capacities" scenario is a generic blanket.** multi-client-and-tracing.feature.coverage line ranges 73-122 map the entire 1-50 region of `codelet/rpc/src/lib.rs`. The actually-relevant lines are 81-90 (const declarations) and 134-135 / 155-156 (the `broadcast::channel(...)` call-sites). Re-link to those specific ranges.

## 🟢 Observations (Nice to Have)

1. The hard-coded `#[ignore]` ceiling of 10 (regression_invariants_rpc011.rs:268) is brittle. Consider snapshotting per-file counts so a legitimate new ignore in another card doesn't accidentally trip RPC-011's regression net.

2. The custom `CaptureLayer` (multi_client_and_tracing_rpc011.rs:192-257) is non-trivial test infra. Promoting it to `codelet/rpc-server/tests/common/mod.rs` would let future tracing-spec tests reuse it.

3. The chunk-equality assertion uses `bincode::serialize` (multi_client_and_tracing_rpc011.rs:112-116) to compare two `Vec<StreamChunk>`. A direct `assert_eq!(chunks_a, chunks_b)` (assuming `StreamChunk: PartialEq`) would produce a clearer diff on failure.

4. The `bind_and_serve` source-shape check (regression_invariants_rpc011.rs:44-47) requires the exact substring `(SocketAddr, ServerStats, tokio::task::JoinHandle<()>)`. A `rustfmt` reformat that put `tokio::task::JoinHandle<()>` on a new line would break the test even though the function remains semantically identical. Consider regex-based matching that tolerates whitespace.

5. `multi-client-and-tracing.feature` uses `Background:` only as a user-story heading without body steps. This is conventional in the project but means workspace/watcher setup is duplicated across the test functions.

6. The tracing test silently swallows the "already-set global subscriber" error (multi_client_and_tracing_rpc011.rs:268-273). Within one cargo test binary cargo serializes `#[tokio::test]` functions, so this is currently safe — but if a future contributor adds another in-process tracing-capture test, only the first will capture and the second's assertions may spuriously fail.

7. RPC-011 rule [21]'s `connect_with_supervisor` is verified as "exists" but not as "preserves additivity contract." A behavioural test that calls BOTH constructors against the same daemon and asserts identical RPC results would prove additivity beyond source-shape.

## Coverage Verification

- Feature file (multi-client): `spec/features/multi-client-and-tracing.feature` — OK; 3 scenarios; `@RPC-011` tag present; doc-string architecture note present; G/W/T ordering correct in all 3 scenarios.
- Feature file (regression): `spec/features/rpc011-regression-invariants.feature` — OK; 4 scenarios; `@RPC-011` tag present; doc-string present; G/W/T ordering correct.
- Test file: `codelet/rpc-server/tests/multi_client_and_tracing_rpc011.rs` — **ISSUE**: header references nonexistent `fspec-binary-polish-rpc011.feature`; 17 `@step` comments present but two have extra word "fspec"; tracing scenario uses surrogate assertions for pinned-port Gherkin; tautological `||` filter at line 312.
- Test file: `codelet/rpc-server/tests/regression_invariants_rpc011.rs` — **ISSUE**: same stale header; @step comments otherwise match Gherkin verbatim; several assertions are weak (commas ≤ 1, `action_tx` substring, comment-bypass on 127.0.0.1, Envelope-sole-type unproven, "tests still pass" is file-existence).
- Impl file: `codelet/rpc/src/lib.rs` — OK; `DEFAULT_CHUNKS_CAPACITY = 1024` (line 81), `DEFAULT_LOGS_CAPACITY = 4096` (line 86), `DEFAULT_WORK_UNITS_CAPACITY = 256` (line 90) all present and used in `broadcast::channel(...)` in both `SharedFspecService::new` (line 134-135) and `with_session_manager` (line 155-156). Matches the Gherkin.
- Impl file: `codelet/rpc-server/src/server.rs` — OK; `bind_and_serve` (line 79-82) preserves the RPC-005 signature `(bind_addr: &str, service: Arc<SharedFspecService>) -> anyhow::Result<(SocketAddr, ServerStats, tokio::task::JoinHandle<()>)>`; `#[tracing::instrument(skip_all, fields(client_id = %peer))]` on `handle_connection` (line 124); enclosing `ws_connection` span with `client_id = %peer` field on the spawned task (line 109).
- Impl file: `codelet/fspec-tui/src/transport/websocket.rs` — OK; `pub async fn connect(url: url::Url) -> Result<Self>` preserved (line 60); `pub async fn connect_with_supervisor(url, action_tx)` added beside it (line 87) — confirmed additive.
- Scenario coverage: 3/3 covered in multi-client-and-tracing; 4/4 covered in rpc011-regression-invariants.

## Files Reviewed

- `/Users/rquast/projects/fspec/spec/features/multi-client-and-tracing.feature`
- `/Users/rquast/projects/fspec/spec/features/multi-client-and-tracing.feature.coverage`
- `/Users/rquast/projects/fspec/spec/features/rpc011-regression-invariants.feature`
- `/Users/rquast/projects/fspec/spec/features/rpc011-regression-invariants.feature.coverage`
- `/Users/rquast/projects/fspec/codelet/rpc-server/tests/multi_client_and_tracing_rpc011.rs`
- `/Users/rquast/projects/fspec/codelet/rpc-server/tests/regression_invariants_rpc011.rs`
- `/Users/rquast/projects/fspec/codelet/rpc-server/tests/common/mod.rs`
- `/Users/rquast/projects/fspec/codelet/rpc-server/src/server.rs`
- `/Users/rquast/projects/fspec/codelet/rpc-server/src/client.rs` (chunks_rx / FspecWsClient / ws_client_connect surfaces)
- `/Users/rquast/projects/fspec/codelet/rpc/src/lib.rs`
- `/Users/rquast/projects/fspec/codelet/fspec-tui/src/transport/websocket.rs` (lines 1-120)
- `/Users/rquast/projects/fspec/codelet/rpc-embedded/tests/architecture_invariants.rs` (scenario list verification)
- Directory listing of `/Users/rquast/projects/fspec/spec/features/` (verifying absence of `fspec-binary-polish-rpc011.feature`)
- Workspace grep for `tokio::runtime::Builder` / `Runtime::new()` in `codelet/rpc-server/src/` and `codelet/fspec/src/`
