# Epic Review: RPC-382 — Port AgentView turn content modal (Enter on selected turn) to Rust

**Date:** 2026-06-28
**Reviewer:** Claude Code (fspec review skill) + subordinate reviewer agent
**Work Units Reviewed:** 1 (standalone story, depends on RPC-381)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2
- 🟢 Observations: 4
- Overall status: **PASS** (warnings addressed below)

## Work Unit Results

### RPC-382: Port AgentView turn content modal — PASS

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (Should Fix)
1. **Tab tear-down does not clear `turn_modal_seq` view-locally.** `views/agent/dispatch.rs`
   Tab branch flips `self.turn_select_mode` locally but clears the modal only in the reducer
   (`app/dispatch_scroll.rs:29-32`). Correct for the App path (and the Tab test passes), but
   it breaks the RPC-381 symmetry (design §4.7) where the view reflects mode changes locally
   so a standalone `AgentView` stays consistent. Esc tear-down IS view-local
   (`dispatch_select.rs:53`). → Add `self.turn_modal_seq = None;` in the dispatch.rs Tab
   branch when disabling select mode.
2. **Coverage impl-line attribution drift for the Esc-cascade scenarios.** "A second Esc
   after closing the modal exits turn-selection mode" was linked to
   `dispatch_select.rs:52-59` (includes the modal-open branch 52-55 NOT taken in that
   scenario) and omits the reducer arm `handle_toggle_turn_select_mode`
   (`dispatch_scroll.rs:27-39`) that performs the exit. Cosmetic (behaviour fully tested). →
   Re-link to the exit branch + the reducer arm.

#### 🟢 Observations (Nice to Have)
1. Pure-harness test helpers (`key`/`tab`/`drain_app`/`render_rows`/`sid`) are triplicated
   across the RPC-381/382 test modules; a shared `tests/common/agent_select.rs` could de-dup
   (not the seed fns, which legitimately differ).
2. `kind_for_seq` clones `ChunkKind` per render frame — negligible.
3. Modal correctly reuses `dialog_theme::render_dialog` + `text_wrap::wrap_to_width` — no
   reinvented overlay/wrap logic. Good DRY/SOLID.
4. In-modal scrolling deferred per design §6 (body clipped to height) — correct scoping;
   candidate follow-up card for very long turns.

#### Coverage Verification
- Feature file: `spec/features/agentview-turn-content-modal.feature` — OK (G/W/T ordering,
  no placeholders, architecture doc-string, `@RPC-382` + `@tui-component`/`@agent-view`/
  `@rust` tags; `@wip` until done).
- Test files: `tests/turn_content_modal_rpc382.rs`,
  `tests/common/turn_content_modal_rpc382_helpers.rs` — OK (@step text exact; real
  full-vs-collapsed assertions).
- Impl files: `turn_modal.rs` (new), `agent.rs`, `dispatch_select.rs`, `dispatch.rs`,
  `scrollback_select.rs`, `components/mod.rs`, `app/dispatch.rs`, `app/dispatch_scroll.rs`
  — OK (end-to-end wired; no unwrap/panic/todo; all < 300 lines; clippy-clean except
  pre-existing tui093).
- Scenario coverage: 6/6.

## Build & Test (verified by supervisor)
- `cargo build -p codelet-fspec-tui`: OK
- `cargo test -p codelet-fspec-tui`: all suites pass (RPC-382 6/6; full suite green)
- `cargo clippy -p codelet-fspec-tui --lib --tests`: only pre-existing tui093 warnings
- `cargo fmt -p codelet-fspec-tui -- --check`: clean

## Fix Results
- 🟡 W1 → ✅ Fixed: added view-local `turn_modal_seq = None` clear in the dispatch.rs Tab
  tear-down branch (restores the RPC-381 view-local mirror invariant). All tests still pass.
- 🟡 W2 → ✅ Fixed: re-linked "A second Esc…" coverage to the Esc exit branch
  (`dispatch_select.rs:57-59`) + reducer arm (`app/dispatch_scroll.rs:27-39`).

## Final Verification (post-fix)
- `cargo test -p codelet-fspec-tui`: all pass.
- `cargo clippy`/`cargo fmt --check`: clean (except pre-existing tui093).
- `audit-coverage agentview-turn-content-modal`: valid; 6/6 scenarios covered.
