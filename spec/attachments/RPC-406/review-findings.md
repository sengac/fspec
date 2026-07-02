# Review Findings: RPC-406 — Inline tool-approval pause prompt in input area

**Date:** 2026-07-02
**Reviewer:** Claude Code (fspec review skill, parallel reviewer eb46264f)
**Status:** WARN (0 critical, 4 warnings, 6 observations)

## 🔴 Critical Issues
None. All security-critical parity requirements verified:
- Esc→Deny on Triple (`pause_keys.rs:64-70`, test asserts `pause_triple(Deny)` once AND `pause_resume_calls()==0`)
- Esc→confirm(false) (`pause_keys.rs:81-87`)
- `Action::PauseResumed` has NO construction site anywhere in `src/` (source-shape locked)
- Prompt actions target the PAUSED session id via render-time `last_pause` cache
- Modal fully removed (no `pause_dialog.rs`, `OpenPauseDialog`, `PAUSE_DIALOG_ID` in `src/`)
- Tests: `inline_pause_prompt_rpc406` 18/18 pass; `pause_hitl_rpc053` 15/15 pass

## 🟡 Warnings (Should Fix)
1. **RPC-053 coverage line ranges are whole-file, not scenario-scoped** — `pause-and-hitl-dialogs.feature.coverage` links every scenario to `dispatch_pause_hitl.rs:1-298` or `hitl_dialog.rs:1-427` wholesale (5,502 "implementation lines" for 15 scenarios). Re-link each scenario to its specific helper fn (e.g. `handle_pause_chunk` 55-112, `handle_pause_cleared` 117-121).
2. **RPC-406 coverage impl ranges partially misattributed** — "Esc on a triple prompt denies…" links `pause_keys.rs:38-93` (whole handler) but omits `dispatch_pause_hitl.rs::handle_pause_triple` (190-209, the actual deny write + slot clear); "Y approves and N denies…" links `pause_keys.rs:76-90` but omits `handle_pause_confirmed` (166-186). Add the missing impl links.
3. **Triple header row clips instead of wrapping** — `render_pause_prompt` puts prompt + details on ONE row; a long prompt/path is truncated by `Paragraph` with no ellipsis or wrap. TS Ink wraps. Fix: wrap the triple header (and confirm header) across rows and make `prompt_height` account for wrapped lines given the area width.
4. **Gherkin ordering** — scenario "Y approves and N denies a confirm prompt" (feature line ~136) has a `Given` after `Then` (second precondition mid-scenario). Restructure (split into two scenarios or rephrase as When).

## 🟢 Observations (Nice to Have)
1. `pause_state.rs` uses `is_none_or` (pins Rust ≥1.82) — fine.
2. `clear_pause_state` inserts `0` into `triple_pause_selection_by_session` instead of `remove()` — permanent map entry; behavior identical. Cleaner to remove.
3. Source-shape lock is a string `contains` check — acceptable.
4. Ctrl+C handling consistent with main dispatch path.
5. Draft-preservation structurally sound (early-return before `sync_viewport`; keys swallowed before `handle_event_gated`).
6. All new files carry feature-file header references.

## Coverage Verification
- Feature file: spec/features/inline-tool-approval-pause-prompt.feature — OK (18 scenarios, @RPC-406, arch doc string accurate; W4 noted)
- Test files: tests/inline_pause_prompt_rpc406.rs — OK (1:1 mapping, exact @step text, behavioral assertions); tests/pause_hitl_rpc053.rs — OK (pause scenarios rewritten to store slot, HITL intact)
- Impl files: pause_state.rs (80 LoC), pause_prompt.rs (151), pause_keys.rs (93), input_area.rs (83), dispatch_pause_hitl.rs (298), dispatch.rs (220), components/mod.rs — OK; end-to-end wiring verified from chunk to render to keys; cursor gate at agent.rs:178-184
- Scenario coverage: 18/18 (RPC-406) + 15/15 (RPC-053) — precision issues per W1/W2

## Fix Results (2026-07-02, remediation worker 2fc6fd5f)

- **W1 (whole-file RPC-053 coverage) — FIXED.** Every `pause-and-hitl-dialogs.feature` scenario re-linked from the blanket `dispatch_pause_hitl.rs:1-298` / `hitl_dialog.rs:1-427` ranges to its specific helper(s), verified against current sources: chunk scenarios → `handle_pause_chunk` 55-112 (+ `handle_open_hitl_dialog` 154-161 where a dialog opens); Idle-pop → `handle_pause_cleared` 117-121; send-error → `handle_hitl_submitted` 234-252; dialog scenarios → `handle_event` 249-319 plus `submit_option` 144-162 / `submit_free_text` 164-179 / `esc_and_pop` 181-187 / `move_selection` 129-135 / `rows_for_render` 189-238 + `render` 320-330 as applicable. Only the source-shape scenario ("hosts the new pause/HITL helpers") keeps the whole-file range, which is what it asserts. 15/15 covered, audit clean.
- **W2 (missing impl links) — FIXED.** "Esc on a triple prompt denies…" now also links `dispatch_pause_hitl.rs::handle_pause_triple` 190-209; the confirm scenarios (split per W4) each also link `handle_pause_confirmed` 166-186 (line numbers verified before linking). "Esc on a confirm prompt denies" gained the same 166-186 link.
- **W3 (triple header clips) — FIXED (full ACDD, red-green).** New scenario "Long triple prompt header wraps instead of clipping" + failing test (44-col terminal, long prompt+details → asserts 2+ header rows, byte-exact no character loss, options row + hint below), then implemented: `pause_prompt.rs` gained `header_spans` + `wrap_styled` (style-preserving char-slice wrap, char-count width proxy consistent with `text_wrap.rs`); both Triple and Confirm headers wrap; `prompt_height(state, width)` now counts wrapped header rows for BOTH kinds; `input_area.rs::input_area_height` takes the full area width and derives the pause body width (area − 2 side pads) mirroring `paint_input_area`; `views/agent.rs` caller updated. All files remain <300 lines (pause_prompt.rs 207, input_area.rs 97).
- **W4 (Given-after-Then) — FIXED.** "Y approves and N denies a confirm prompt" split into "Y approves a confirm prompt" and "N denies a confirm prompt" (clean Given/When/Then each); the merged test split into two tests with 1:1 `@step` comments; coverage re-linked (20/20 scenarios).
- **Obs2 (selection map leak) — FIXED.** `clear_pause_state` now `remove()`s the `triple_pause_selection_by_session` entry instead of inserting 0 (unset reads as 0); feature arch doc string updated to match.
- **Verification:** `cargo test -p codelet-fspec-tui` all green (20/20 inline_pause_prompt_rpc406 incl. the new wrap test — confirmed red first; 15/15 pause_hitl_rpc053; full crate suite 0 failures); clippy + fmt clean; `fspec validate` OK; audit-coverage clean on both features; coverage 100% on both. Card cycled done → implementing → validating → done.
