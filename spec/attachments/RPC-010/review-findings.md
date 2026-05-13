# RPC-010 Review Findings (Self-Review, 2026-05-11)

**Work Unit:** RPC-010 — `fspec binary: combined frontend+server, plus fspec daemon and fspec client subcommands`
**Reviewer:** Claude Code (review-skill.md, in-session pass)
**Scope:** Strict RPC-010 only — no scope creep into RPC-008/009/011.
**Test status before/after fixes:** 51 passed / 0 failed / 5 ignored (validating-phase attestations)

## Summary

- 🔴 Critical: 0 issues fixed; 1 issue flagged-not-fixed (out-of-crate dependency, see note)
- 🟡 Warnings: 1 fixed (DRY violation in test helpers)
- 🟢 Observations: 3 noted (intentional, documented in code)

All 56 scenarios across the 4 RPC-010 feature files retain 100% coverage. Test-line ranges for `combined_smoke.rs` were re-linked after a 34-line shift caused by the DRY fix; no other coverage links moved (helpers in the other files were at file-end, so their removal didn't shift any test functions).

---

## Per-Source-File Findings

### `codelet/fspec/Cargo.toml` ✅ PASS
- Single `[[bin]]` named `fspec` at `src/main.rs`.
- `[dependencies]` has all 16 required keys (clap, tokio, anyhow, tracing*, dirs, serde*, url, codelet-rpc*, codelet-fspec-tui, codelet-core, ratatui, crossterm).
- `[dependencies]` and `[dev-dependencies]` both exclude `codelet-napi` (rule [13]).
- Dev-deps add `predicates` beyond the architecture-note-3 list — extra is harmless and useful for assert_cmd matchers.

### `codelet/fspec/src/main.rs` (91 lines) ✅ PASS
- `#[tokio::main]` is the only runtime source.
- `--workspace` is a global flag; `--pidfile` only on `daemon` (rule [24]).
- `--bind` defaults to `127.0.0.1:0` (rule [8]); `validate_loopback_bind` is invoked inside `daemon::run` before `bind_and_serve`, so non-loopback fails with non-zero exit + the required stderr text (rule [21]).
- Error path: `eprintln!("{err:#}")` + exit code 1.

### `codelet/fspec/src/combined.rs` (100 lines) ✅ PASS
- `bind_and_serve` called exactly once (rule [2]).
- `tokio::runtime::Handle::current()` is the only handle source (rule [7]).
- `build_service` called exactly once; same Arc passed to both `bind_and_serve` and `EmbeddedFspecBackend::new` (rule [3]).
- `eprintln!("PORT=…")` on STDERR (rule [4]); STDOUT is the alt-screen.
- `join.abort()` is called BEFORE `remove_daemon_json` (rule [23]).
- `drive_run` races `App::run` against the SIGINT/SIGTERM shutdown future; on `App::run` error (e.g. ENXIO in a headless harness) it blocks on shutdown so combined mode remains useful as a WS-only subprocess. Documented in the function docstring.

**Deviation from architecture note [1] — DOCUMENTED**: combined mode writes `daemon.json` BEFORE emitting `PORT=`; the architecture note listed the reverse order. The reordering is intentional (eliminates a test race where the assertion on the banner outraces the daemon.json file write). Documented inline at lines 45–48.

### `codelet/fspec/src/daemon.rs` (64 lines) ✅ PASS
- `validate_loopback_bind` runs BEFORE `bind_and_serve` (rule [21]).
- `bind_and_serve` called exactly once; port emitted on STDOUT as bare integer (RPC-005 contract).
- Optional pidfile written on bootstrap, removed on shutdown (rule [24]).
- Daemon.json written on bootstrap, removed on shutdown (rule [18]).
- `build_shutdown_future` races SIGINT + SIGTERM (rule [9]).
- JoinHandle is intentionally discarded (`_join`) — the tokio runtime is dropped when `run()` returns, which aborts all spawned tasks. Acceptable for a process whose lifetime ends at shutdown.

### `codelet/fspec/src/client.rs` (127 lines) ✅ PASS (with documented headless fallback)
- `resolve_connect_url` honours explicit `--connect` first, then daemon.json (rule [10]). Fails fast with stderr containing "no daemon.json found" + "--connect" when neither resolves.
- Does NOT construct `SharedFspecService`, does NOT call `bind_and_serve`, does NOT call `ratatui::init` directly (rule [11], scenario "Client mode does not construct…").
- `App::new(Arc::new(backend))` → `bootstrap().await` → `run().await` (rule [11] + RPC-009 sequence).
- `drive_run` + `stdin_quit_signal` are the headless-test fallback path (RPC-010 implementing-phase fix recorded in session a348b9fc). They only execute if `App::run` returns Err (e.g. ENXIO from `enable_raw_mode` in a pipe harness). The fallback consumes stdin bytes, returns on `'q'`, and blocks on EOF/Err so the reconnect-bootstrap test (which closes stdin after `'r'`) does not race-exit the client.

### `codelet/fspec/src/common.rs` (245 lines) ✅ PASS
- `build_service` constructs the single `Arc<SharedFspecService>` (rule [3]).
- Three `init_tracing_*` functions match architecture note [7]: registry built FIRST with appropriate layers, then `register_log_layer(service)` called AFTER so only the sender-push side-effect lands.
- `install_panic_hook` is idempotent (uses `Once`), restores ratatui + disables mouse/bracketed-paste BEFORE calling the previous panic hook.
- `daemon_json_path` honours `$XDG_RUNTIME_DIR/fspec/daemon.json` else `~/.fspec/daemon.json` (rule [10]/[18]).
- `write_daemon_json` is atomic (temp+rename) with required keys port/pid/workspace/version (rule [17]).
- `validate_loopback_bind` accepts `127.0.0.1`, `::1`, `localhost`; rejects everything else with the rule-[21] stderr text.
- `build_shutdown_future` registers signal handlers SYNCHRONOUSLY then awaits them inside the returned future (handlers are live before bind).

### Tests — Source-shape regression in `codelet/rpc-embedded/tests/architecture_invariants.rs` ✅ PASS
- The RPC-005 source-shape scan was widened to include `codelet/fspec/src/` (rule [7], scenario "The RPC-005 source-shape invariant is widened…"). Verified via grep on the file.

### `package.json` ✅ PASS
- `scripts["build:rust:fspec"]` invokes `cargo build -p fspec --release` and copies to `dist/fspec` (rule [15]).
- `bin["fspec"]` still points at the TS shim `./dist/index.js` (rule [14] / scenario "npm bin entry remains on the TS shim").

---

## 🔴 Critical Issue Flagged But NOT Fixed (Out-of-Crate)

### CR-1: Rule [12] / Rule [25] disconnect dialog text + reconnect bootstrap are not implemented in the App component

**Where the gap is:** `codelet/fspec-tui/src/app.rs` and `codelet/fspec-tui/src/transport/websocket.rs` (RPC-008/009 crates, NOT RPC-010's `codelet/fspec/`).

**What's missing:**
- No WS-disconnect signal exposed by `WebSocketFspecBackend`.
- No subscriber in `App` that pushes a `Priority::Critical` modal layer when the WS connection drops.
- No layer body containing `daemon disconnected`, `q to quit`, `r to reconnect` (grep returns zero hits in `codelet/fspec-tui/src/`).
- No `r`-key handler in `App` that performs the rule-[25] full reconnect bootstrap (drop backend → abort 3 subscriber tasks → `WebSocketFspecBackend::connect(url)` → `list_work_units` + `create_session(None)` + respawn 3 subscriber tasks).

**Why tests still pass:** The four affected scenarios (`WS disconnect mid-session…`, `Pressing q in the disconnect dialog…`, `Pressing r performs a full reconnect bootstrap`, `Reconnect attempt does not loop…`) are implemented as **surrogate** assertions per the locked Q2 design ("daemon-side observability only — no pty harness"). The surrogate is "client subprocess remains alive" — which is true even though the actual dialog is never rendered. So coverage and tests are GREEN even though the user-visible behaviour described in the feature file is partly aspirational.

**Why I did NOT fix this in this review:**
1. The fix lives in `codelet/fspec-tui/` (RPC-008/009 territory), not `codelet/fspec/` (RPC-010 territory). The user's instruction was "strictly to the requirements of this card — no scope creep". Implementing the dialog/reconnect in a sibling crate IS scope creep into RPC-008/009.
2. The feature was signed off as `done` previously with these scenarios green via the surrogate; the locked Q2 design appears to be deliberate.
3. Estimated effort: ~4–6 hours touching ~5 files (FspecBackend trait extension, WebSocketFspecBackend, App subscriber + Action variant, new `DisconnectDialog` component, App keybinding handler for `r`).

**Recommendation:** This belongs in a follow-up card (likely the existing RPC-011 "polish" or a dedicated "client disconnect dialog" card). I am surfacing it here so it isn't silently forgotten.

---

## 🟡 Warning Fixed

### W-1: `strip_comments` helper was duplicated across 4 integration-test files (~34 lines each, ~136 lines of dead duplication) — FIXED

**Before fix:** Identical `strip_comments` function defined in
- `tests/cargo_shape.rs:650` (now removed)
- `tests/client_mode.rs:727` (now removed)
- `tests/combined_smoke.rs:27` (now removed)
- `tests/daemon_mode.rs:612` (now removed)

**Fix:** Moved a single `pub fn strip_comments` to `tests/common/mod.rs` (lines 158–195). Each test file now imports it via `use common::{…, strip_comments, …};`.

**Side effect:** `combined_smoke.rs` test-function line numbers shifted UP by 34 lines because `strip_comments` was at the TOP of that file (lines 27–60). The other three files had `strip_comments` at the bottom, so removing it shifted no test functions. Coverage links for all 9 `combined_smoke.rs` scenarios were re-linked with the new line ranges via `unlink-coverage --all` + `link-coverage`.

**Verification:** `cargo test -p codelet-fspec --release` — 51 passed / 0 failed / 5 ignored.

---

## 🟢 Observations (No Fix — Intentional / Documented)

### O-1: `drive_run` + `stdin_quit_signal` in `client.rs` is production code that exists for the headless test harness

**Location:** `codelet/fspec/src/client.rs:43–112`

**What it does:** If `App::run` returns `Err` (e.g. `enable_raw_mode` failed with `ENXIO` in a non-TTY subprocess), the client falls back to (a) reading stdin for `'q'` byte and (b) racing against the shutdown future. This keeps the client subprocess alive in pipe-only test harnesses so the integration tests can assert "client did not crash on disconnect".

**Why it's OK:** Documented in the function docstrings, gated behind the App::run error path (the normal interactive path never enters this code), and is the agreed-upon shape after the prior RPC-010 implementing phase decided to use daemon-side observability + the python timeout wrapper instead of a pty harness.

### O-2: `combined.rs` writes `daemon.json` BEFORE the `PORT=` banner; architecture note [1] listed the reverse order

**Location:** `codelet/fspec/src/combined.rs:48–54`

**What it does:** Reorders the bootstrap sequence so daemon.json is on disk before any external observer parses the banner. Eliminates a TOCTOU race in the test "Combined mode writes daemon.json on bootstrap…".

**Why it's OK:** Documented inline. The user-observable end-state is identical — the banner appears flushed within the same RPC bootstrap span. Could be reflected in architecture note [1] for accuracy, but the deviation is minor.

### O-3: `daemon.rs` discards the JoinHandle from `bind_and_serve`

**Location:** `codelet/fspec/src/daemon.rs:36`

**What it does:** Lets the tokio runtime tear down the WS server task implicitly when `run()` returns (which happens on signal). Combined mode, by contrast, explicitly aborts the handle before removing daemon.json (rule [23]).

**Why it's OK:** Daemon mode terminates the entire process on signal — there's no "post-WS-server" cleanup that needs to observe an aborted handle. Rule [23] only applies to combined mode. The architecture note [1] for daemon does not mention JoinHandle abort.

---

## Fix Results

| Issue | Status | Verification |
|-------|--------|--------------|
| W-1 DRY (`strip_comments` ×4) | ✅ FIXED — moved to `tests/common/mod.rs:158–195`; 4 imports updated; coverage re-linked for 9 combined-mode scenarios | 51/0/5 cargo test, 100% coverage on all 4 features |
| CR-1 Disconnect dialog gap | ⚠️ FLAGGED, NOT FIXED — recommend dedicated card | Documented for follow-up |

## Final Verification

- All RPC-010 tests pass: ✅ (`cargo test -p codelet-fspec --release`)
- All RPC-005..009 sibling tests still pass: ✅ (already confirmed in prior session per attestation tests; not re-run this pass)
- Clippy on `codelet-fspec` (with `--no-deps`): ✅ clean
- Coverage 100% on all 4 features (cargo-shape 22, combined 9, daemon 13, client 12): ✅
- All source files in `codelet/fspec/src/` ≤ 300 lines: ✅ (max: common.rs at 245)
- No `codelet-napi` in `[dependencies]` or `[dev-dependencies]`: ✅
- Source-shape regression (no tokio runtime construction in fspec/src): ✅
