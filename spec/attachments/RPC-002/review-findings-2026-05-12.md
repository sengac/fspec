# Epic Review: RPC-002 — Rust ratatui frontend with dual transport (codelet/fspec-tui)

**Date:** 2026-05-12
**Reviewer:** Claude Code (fspec review skill — parallel ACDD compliance review)
**Work Units Reviewed:** 7 (RPC-005, RPC-006, RPC-007, RPC-008, RPC-009, RPC-010, RPC-011)
**Mode:** Read-only / discovery (precursor to new refactor story, not a fix run)

## Summary

| Card | Title | Status | 🔴 Critical | 🟡 Warning | 🟢 Observation |
|------|-------|--------|------------|-----------|----------------|
| RPC-005 | Foundation: dual-transport tarpc service | PASS (WARN) | 0 | 4 | 10 |
| RPC-006 | Real work-units backing + WorkUnitsUpdate envelope | PASS (WARN) | 0 | 5 | 13 |
| RPC-007 | Session RPCs + StreamChunk/LogEvent push channels | WARN | 0 | 7 | 8 |
| RPC-008 | FspecBackend trait + Compositor + app shell | WARN | 0 | 6 | 8 |
| RPC-009 | Basic frontend — work-units list + agent REPL | **FAIL** | **3** | 7 | 7 |
| RPC-010 | fspec binary (combined / daemon / client) | WARN | 0 | 6 | 11 |
| RPC-011 | Reconnect, daemon lifecycle, multi-client polish | PASS | 0 | 7 | 15 |

**Totals:** 🔴 3 Critical · 🟡 42 Warnings · 🟢 72 Observations

## What Already Exists (informs RPC-002 refactor scope)

### Transport seam (RPC-005..008) — STABLE
- `codelet-rpc-types` — canonical serde types; `codelet-napi` re-exports.
- `codelet-rpc` — `FspecService` tarpc trait; `SharedFspecService`; `ArcSwap<WorkUnitsWatcher>`.
- `codelet-rpc-embedded` — `EmbeddedTransport` over `tarpc::transport::channel`.
- `codelet-rpc-server` — `bind_and_serve` (127.0.0.1 only) + bincode `Envelope` codec.
- `codelet-fspec-tui::FspecBackend` trait — single async surface every UI consumer uses.
- `EmbeddedFspecBackend` + `WebSocketFspecBackend` (latter with reconnect supervisor).

### App shell (RPC-008..009) — MINIMAL
- `App` (codelet/fspec-tui/src/app.rs) — Compositor (modal layers) + `RootView` (always-on background).
- `RootView` — horizontal split: `WorkUnitsListView` (LEFT 50-col) | `AgentReplView` (RIGHT) + 1-row `FooterView`.
- `WorkUnitsListView` — **flat ratatui `List`** of `{id} [{status}] {title}`; j/k navigation; no columns, no details panel.
- `AgentReplView` — single-session scrollback (`Vec<RenderedChunk>`) + `tui_input::Input` line; Tab cycles focus.
- `Action` enum: Quit, Redraw, Custom, LoadWorkUnits, WorkUnitsLoaded, SessionCreated, ChunkReceived, InputSubmitted, Interrupt, FocusNext, plus RPC-011's Disconnected/Reconnecting/Reconnected/ManualReconnect.

### Binaries & lifecycle (RPC-010..011) — SOLID
- `fspec` (combined: TUI + WS server), `fspec daemon`, `fspec client`, `fspec status`.
- `daemon.json` autodiscovery + stale-PID kill-probe.
- Reconnect supervisor with 250ms→5s exponential backoff (in `transport/websocket.rs`).
- `DisconnectDialog` at `Priority::Critical` with `q`/`r` honoured.
- SIGTERM graceful drain + going_away Close frame; SIGHUP watcher rebuild.
- `health()` RPC + `ServerStats` with `connected_clients`, lag counters, ArcSwap watcher.

### Tests
- 100% scenario coverage with @step comments across all RPC-005..011 features (with notable exceptions in RPC-009).
- Source-shape regression in every layer: no own-runtime construction, no codelet-napi in non-NAPI crates, loopback bind enforcement, runtime-Handle invariant.

## 🔴 Critical Issues Found (RPC-009 only)

1. **`RootView` left-pane width is 50 cols, contract says 32.**
   - `codelet/fspec-tui/src/views/root.rs:122`: `Constraint::Length(50)`.
   - Rule [10], feature `fspec-tui-root-layout-rpc009.feature:27`, in-file tests at `root.rs:192/201` all mandate `Length(32)`.

2. **Work-units list render format diverges from contract.**
   - Rule [4] / architecture note [7] spec `format!("{} {}", id, status)`.
   - Implementation at `work_units_list.rs:145` renders `format!("{} [{}] {}", id, status, title)` (brackets + title).
   - Tests at `work_units_list.rs:230, 232, 252, 261, 400, 402` assert the bracketed form — feature-test divergence.

3. **`Action` enum lost `PartialEq, Eq` derives.**
   - Architecture note [3] requires them; `components/mod.rs:85` has only `#[derive(Debug, Clone)]`.
   - Feature scenario at `fspec-tui-app-bootstrap-rpc009.feature:97` was updated to ACCEPT the drop — rule and feature contradict.

These are **historical drifts during done-state work**; not blockers for new work but should be noted before RPC-002 refactor builds on them.

## 🟡 Recurring Warnings (cross-cutting)

1. **File-size ceiling (300 LoC) is breached in 6+ production files**, mostly because successive cards layered onto the same files:
   - `codelet/fspec-tui/src/app.rs` — **604 LoC** (App = bootstrap + dispatch + run + 3 subscriber tasks + RPC-011 reconnect handling)
   - `codelet/fspec-tui/src/transport/websocket.rs` — **399 LoC**
   - `codelet/fspec-tui/src/views/agent_repl.rs` — **519 LoC** (incl. inline #[cfg(test)])
   - `codelet/fspec-tui/src/views/work_units_list.rs` — **423 LoC**
   - `codelet/fspec-tui/src/views/root.rs` — **328 LoC**
   - `codelet/rpc/src/lib.rs` — **394 LoC** ; `codelet/rpc-types/src/lib.rs` — **698 LoC**
   - `codelet/rpc-server/src/server.rs` — **303 LoC**
   - `codelet/fspec/src/common.rs` — **463 LoC**

2. **`FspecBackend` trait surface drift vs RPC-008 rule [1].** Trait now exposes 6 RPC methods (added `health`) + `request_manual_reconnect()`. RPC-008 feature still says "exactly five RPC methods + 3 broadcast subscriptions"; source-shape test does not enforce the new surface.

3. **Coverage line-range bloat.** Many `.feature.coverage` entries list whole-file ranges (e.g. `rpc/src/lib.rs:1-139`) rather than narrow impl ranges. Cosmetic, not blocking.

4. **`Feature:` headers in some RPC-007 test files reference a non-existent split feature file.**

5. **Stale Gherkin text in RPC-008's `fspec-tui-app-shell.feature`** — still says "App::new pre-populates the compositor with a Background-priority HelloComponent", which RPC-009 superseded by `RootView`.

## 🟢 Strong Points

- Architecture invariants from RPC-005 (Q9 host-supplied runtime Handle, single source of truth for types, loopback bind, no codelet-napi in non-NAPI crates) are **rigorously enforced via source-shape regression tests** in every layer.
- Both transports remain at parity — every RPC tested through both paths.
- `Component` + `Compositor` + priority model is clean, faithful to RPC-002 doc 09.
- `tokio::spawn`-only async (never `Runtime::new`/`Builder`); subscriber tasks honour `RecvError::Lagged`.
- `TerminalGuard` RAII with idempotent panic-hook restoration.
- ACDD coverage discipline is high — every scenario @step-linked to a real Rust test that asserts the actual behaviour.

## Structural Gap vs TypeScript TUI (informs new refactor story)

The TS reference TUI (`src/tui/components/UnifiedBoardLayout.tsx` + `src/tui/components/AgentView.tsx`) provides two top-level views that the Rust TUI does NOT yet model:

| TS surface | Rust today | Gap |
|------------|-----------|-----|
| `UnifiedBoardLayout` — 7-column Kanban (backlog→done+blocked), per-column scroll, work-unit details panel, [/] reorder, 🟢 session indicator, ⏩ last-changed | `WorkUnitsListView` — flat list, j/k, no columns, no details, no session attachment | New columnar Board view + WorkUnit details panel |
| `AgentView` — full-screen, multi-session, streaming chunks, slash commands, model picker, thinking levels, history, navigation target | `AgentReplView` — single-session, single-line `tui_input`, scrollback `Paragraph` | Eventual full AgentView port (large; the refactor story unlocks it, not delivers it) |
| `useFspecStore` (Zustand) — work units, epics, file status, checkpoint counts, session attachments | All state in component fields on each view | **No store layer** — every view owns its own slice |
| `useSessionStore` (Zustand) — current session, navigation target (BoardView → AgentView), isolation, debug, dialog visibility, work-unit↔session mapping | `AgentReplView.active_session: Option<SessionId>` only | **No navigation state** — no Board↔Agent handoff, no nav-target session id |
| BoardView Enter on work unit → AgentView attached to it · Shift+Right → first attached session or create-dialog | Tab cycles focus between list+REPL; no view switching | **No top-level navigator** — App's RootView is fixed two-pane |

The new story below addresses ONLY the **base store refactor** + RootView restructure into a navigator that can switch between BoardView and AgentView. Full Kanban rendering, work-unit details, full AgentView port etc. are downstream slices that consume the new stores.

## Files Reviewed

Per-card review findings are summarised above. Source files inspected:
- `codelet/{rpc-types,rpc,rpc-embedded,rpc-server}/src/**`
- `codelet/fspec-tui/src/{app.rs,compositor.rs,lib.rs,components/**,transport/**,views/**}`
- `codelet/fspec/src/{main,combined,daemon,client,common,status}.rs`
- All 36 RPC-005..011-linked `spec/features/*.feature` files
- All corresponding `*.feature.coverage` files
- All RPC-005..011 integration tests under `codelet/**/tests/`

## Next Steps (not a fix run)

The 🔴 RPC-009 critical issues (left-pane width 50 vs 32, `[status]` brackets, lost PartialEq derive) should be ABSORBED into the new RPC-002 refactor story rather than fixed in isolation — the refactor will rewrite RootView and the work-units rendering anyway. The new story's first slice should re-state the contract (Length(32) or new width tuned for Board columns; render format chosen for the new view; Action enum derives explicit) so the old drift becomes moot.

The other 🟡 warnings (file sizes, coverage bloat, stale feature headers) are cosmetic and tracked in their respective cards' attached `review-findings.md` files; not blocking for the refactor.
