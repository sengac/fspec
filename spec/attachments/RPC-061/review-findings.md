# Epic Review: RPC-061 — Supervisor / subordinate links surface

**Date:** 2026-05-25
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (parent, no children — RPC-030 is the umbrella)
**Feature files in scope:** 3
  - spec/features/rpc061-cross-transport-parity.feature
  - spec/features/rpc061-source-shape.feature
  - spec/features/rpc061-supervisor-links.feature

## Summary
- 🔴 Critical fixed: **3** (rule [9] AttachToSession wiring + missing examples [1]/[7]/[8]
  + wrong feature header citations on two test files)
- 🟡 Warnings fixed: **3** (multi-line @step continuation comments, send-to-subordinate
  Err-without-session test now actually drains the action bus, dispatch.rs same-line
  statement style nit)
- 🟢 Observations acknowledged but not actioned (pre-existing oversize aggregate files
  outside RPC-061's scope, set_supervisor_pending_count YAGNI flag retained for the
  agent-loop reset path described in supervisor_state.rs:18-20)

## Findings — Cross-Transport Parity feature

### 🔴 None.
All 7 scenarios pass through both EmbeddedFspecBackend AND WebSocketFspecBackend against
the same StubSessionManagerHandle. Per-method call counters increment in every method,
BFS cycle detection is correct, and both transports share the stub via
SharedFspecService::with_session_manager.

### 🟡 Fixed
1. Test file header at `codelet/fspec-tui/tests/rpc061_cross_transport_parity.rs:4` was
   citing the wrong feature file (rpc061-supervisor-links.feature). **Fixed** —
   header now reads `Feature: spec/features/rpc061-cross-transport-parity.feature`.
2. Multi-line `@step` continuation comments at lines 89-90 / 96-97 dropped the second
   half of the step text from link-coverage's step extractor. **Fixed** — each step
   is now a single `// @step ...` line.

### 🟡 Acknowledged but not changed (test-strength polish)
- `get_subordinates` "same order" only asserts `em == ws` + length; a strict
  `assert_eq!(em, vec![sub-a, sub-b])` would be stronger.
- `circular_add_supervisor` only asserts the error message; could also re-assert
  stub state is unchanged after the rejected call.
- These are test-tightening opportunities, not correctness bugs — left to a future
  defensive-coding pass.

## Findings — Source Shape feature

### 🔴 None.
All 10 source-shape scenarios pass. Every file pinned by the feature exists, every
member signature matches, and the file-size ceiling for `dispatch_rpc061.rs` (108) and
`app/dispatch.rs` (299) is respected.

### 🟡 Fixed
1. Test file header at `codelet/fspec-tui/tests/source_shape_rpc061.rs:4` cited the
   wrong feature file (rpc061-supervisor-links.feature). **Fixed** — header now reads
   `Feature: spec/features/rpc061-source-shape.feature`.

### 🟡 Acknowledged but not changed
- Internal Rust doc-comments at lines 79/113/147/215/249 use "RPC-061 methods" / "RPC-061
  forwarders" while the feature scenarios say "supervisor methods" / "supervisor
  forwarders". Pure documentation drift; `@step` text already matches the feature
  exactly. Not blocking.

## Findings — Supervisor Links feature

### 🔴 Fixed
1. **Rule [9] partial implementation: AttachToSession + session-cycle paths did not
   spawn `get_supervisors`.** The implementation only fired from
   `Action::SessionCreated`. **Fixed**:
   - `codelet/fspec-tui/src/app/dispatch_rpc026.rs:91-100` now calls
     `self.spawn_load_supervisors(session.clone())` inside
     `handle_attach_to_session`.
   - `codelet/fspec-tui/src/app/dispatch_rpc024.rs:46-53` now calls
     `self.spawn_load_supervisors(incoming_session)` at the end of
     `handle_session_cycle`.
   - `dispatch_rpc061.rs` docstring updated to reflect the three call sites.

2. **Example [1] (no supervisor → no badge) had no scenario.** **Fixed** — added
   scenario "A session with no supervisors shows no subordinate badge" with a
   `format_subordinate_label(&[])` unit test (`tests/supervisor_links_rpc061.rs:332-345`).

3. **Example [7] (multi-supervisor `+N` format) had no scenario.** **Fixed** —
   added scenario "Multi-supervisor session renders subordinate badge with +N count"
   with a `format_subordinate_label` test asserting `Some("s-sup-aa+2")`
   (`tests/supervisor_links_rpc061.rs:351-361`).

4. **Example [8] (supervisor chip wins over compaction chip) had no scenario.**
   **Fixed** — added scenario "Supervisor pending chip suppresses the compaction
   chip" with a render-buffer test that asserts the yellow pending chip is painted
   AND no `[compacting:` substring appears in the rendered row when
   `supervisor_pending_count=1` AND a `CompactionProgress` is present
   (`tests/supervisor_links_rpc061.rs:375-409`).

5. **Rule [9] (AttachToSession reloads supervisors) had no scenario.** **Fixed** —
   added scenario "AttachToSession triggers spawn_load_supervisors and
   SupervisorsLoaded" with an integration test using MockBackend
   (`tests/supervisor_links_rpc061.rs:415-442`).

### 🟡 Fixed
1. **`send_to_subordinate_err_without_open_session_is_silent_no_op` did not actually
   assert the absence of EmitSessionNotice.** The test only checked the
   receive_incoming_message call count. **Fixed** — the test now drains
   `app.try_recv_action()` and asserts no `Action::EmitSessionNotice` is observed on
   the action bus (`tests/supervisor_links_rpc061.rs:200-235`).
2. Multi-line `@step` continuation comments collapsed onto single lines so the
   link-coverage step extractor captures the full step text
   (`tests/supervisor_links_rpc061.rs:112,121,151,186,276`).
3. `dispatch.rs:295` had two statements on a single source line. **Fixed** — split
   onto two lines, file is now 299 LoC (still under the 300 ceiling).

### 🟡 Acknowledged but not changed
- `set_supervisor_pending_count` in `supervisor_state.rs:56-65` is exposed for an
  agent-loop reset path that is not wired yet. The docstring already documents this
  semantic. Out of scope for RPC-061 (Phase 7.8 specifically excludes agent-loop
  changes); will be wired when the agent-loop emits a fresh count chunk.
- `handle_send_to_subordinate` swallows the non-runtime branch silently. This is the
  established pattern across `dispatch_rpc026`/`dispatch_rpc049`/`dispatch_rpc060` so
  unit-test (non-tokio) callers can still drive the dispatcher synchronously.

## Fix Results

### RPC-061: Supervisor / subordinate links surface
- 🔴 Rule [9] AttachToSession not wired → ✅ Fixed
- 🔴 Example [1] no-supervisor → no-badge has no scenario → ✅ Fixed (new scenario + test)
- 🔴 Example [7] multi-supervisor `+N` format has no scenario → ✅ Fixed (new scenario + test)
- 🔴 Example [8] supervisor chip wins over compaction chip → ✅ Fixed (new scenario + test)
- 🔴 Rule [9] AttachToSession path has no scenario → ✅ Fixed (new scenario + test)
- 🟡 send_to_subordinate Err-without-session test was vacuous → ✅ Fixed (drains action bus)
- 🟡 Multi-line @step continuation comments → ✅ Fixed (single-line @step everywhere)
- 🟡 Wrong feature header on cross_transport_parity.rs / source_shape_rpc061.rs → ✅ Fixed
- 🟡 dispatch.rs same-line statement → ✅ Fixed

## Final Verification

- All 28 RPC-061 tests pass (7 cross-transport-parity + 10 source-shape + 11 supervisor-links)
- Full `cargo test -p codelet-fspec-tui` passes — 0 failures across 121+ test suites
- `cargo build -p codelet-fspec-tui` clean
- `fspec validate spec/features/rpc061-supervisor-links.feature` — OK
- `fspec show-coverage rpc061-supervisor-links` — 11/11 scenarios covered
- `fspec show-coverage rpc061-cross-transport-parity` — 7/7 scenarios covered
- `fspec show-coverage rpc061-source-shape` — 10/10 scenarios covered
- File-size audit:
  - `app/dispatch.rs`: 299 LoC (under 300)
  - `app/dispatch_rpc061.rs`: 108 LoC
  - `app/dispatch_rpc024.rs`: 56 LoC
  - `app/dispatch_rpc026.rs`: 228 LoC
  - `store/agent_view/supervisor_state.rs`: 77 LoC
  - `views/agent/header_build.rs`: 207 LoC
  - `views/agent/footer.rs`: 264 LoC
