# Review: MUX-001 — Mux mode

*ACDD compliance review — 2026-08-26. Work unit MUX-001 ("Mux mode — multiplexed top-level views with /mux configuration", status: done).*

## Status: WARN

The implementation is faithful to its own feature file (31/31 scenarios covered, 52/52 tests green, `cargo check` + `cargo clippy` clean), but the spec no longer matches the user's expected behavior (the reported Shift+Right bug is a spec gap, not a code bug), and there are stale references, dead code, and a file-size violation.

## 🔴 Critical Issues (Must Fix)

1. **User-reported bug is a spec gap, not a code bug — the single-agent-pane assumption conflicts with the user's expected behavior.** Example-map assumption 1 ("Mux MVP hosts at most ONE live agent pane (AgentView is a single instance); extra agent panes are out of scope") was the accepted scope for MUX-001, so the code is "correct" against spec — but the user now requires: in a 3-view mux (Board | Agent | Agent), Shift+Right rotates the **agent sessions** among the agent panes while non-agent panes (Board) stay pinned, with all agent views grouped together. The current implementation cannot express this at any layer:
   - `MuxPaneKind::Agent` is a single kind with no per-pane identity (`rust/fspec-tui/src/views/multiplex/mod.rs:34-41`) — a second `agent` pane is parseable (`/mux board agent agent` is not rejected in `mux_parser.rs:105-140`) but has no way to bind a *different* session.
   - `AgentView` is a single instance (`nav.agent`); `forward_to_pane` (`keys.rs:100-102`) routes **every** `Agent` pane to that same view, so two agent panes would render the same session and share one input buffer.
   - `classify_key` (`keys.rs:56-64`) intercepts Shift+Left/Right as **pane-focus cycling** (R3), so the agent's own Shift+←/→ session cycling is dead inside mux; and architecture note 4 documents that crossterm cannot distinguish Ctrl+arrow from Shift+arrow, so the R3 escape hatch ("session cycling moves to Ctrl+Left/Right") is unusable.
   - Changes required to support the user's expectation span: **spec** (new rules + scenarios: per-pane session binding, grouped agent-view cycling with non-agent panes pinned), **config/store** (`MuxConfig.panes` needs per-agent-pane session identity + serde), **layout** (agent-group rotation math), **keys** (Shift+Right must rotate the agent group in place, not move focus), **render/forward** (one `AgentView` per agent pane or per-pane session context in `AgentViewStore`), **tests** (new @step tests). This needs a **new work unit** (e.g. MUX-002) — MUX-001 should not be re-opened in place.

2. **Stale persistence references — `spec/mux.json` vs the correct `tui.mux` in shared `fspec-config.json`.** The authoritative behavior is rule 13 + architecture note 2 + the implementation (`rust/sessions/src/mux_config_persistence.rs`, `store/mux_state.rs`) + the scenarios: config lives in `fspec-config.json` under `tui.mux`, **no** `spec/mux.json`. Stale `spec/mux.json` references remain in:
   - `spec/features/rust-mux-mode.feature:9` (architecture doc string)
   - `spec/features/rust-mux-mode.feature:41` (example 8)
   - `spec/features/rust-mux-mode.feature:51-52` (answered question "Where should the mux config persist to?")
   - `rust/fspec-tui/src/views/multiplex/mod.rs:61` (`MuxConfig` doc comment)
   - `rust/fspec-tui/src/app/mux_parser.rs:46` (`Save` variant doc comment)
   - Work unit MUX-001 description and example-map example [7].
   These contradict rule 13 within the same feature file.

3. **Stale 52-column board minimum in the example map.** Rule [4] ("no pane may be narrower than 52 columns") and example [3] ("clamps it to the 52-col minimum") still carry the pre-correction value. The feature file, architecture note 1, and the implementation (`layout.rs:17`, `MIN_BOARD_PANE_WIDTH = 64`) all agree on **64**.

## 🟡 Warnings (Should Fix)

1. **`mod.rs` is 404 lines** (`rust/fspec-tui/src/views/multiplex/mod.rs`) — over the 300-line ceiling. `recompute_rects` (343-396) is a natural extraction candidate (it also duplicates the divider-rect math in `render.rs:60-73`).
2. **Dead code — mux `Action` variants that are never emitted:** `MuxFocusPrev`, `MuxFocusNext`, `MuxSplitAdjust`, `MuxExit`, `MuxOff`, `MuxConfigApplied` are matched in `dispatch_mux.rs`/`navigator.rs` but no code path ever sends them (only `MuxEnterWorkUnit` is emitted). Also `config_for_panes` (`dispatch_mux.rs:183`) has zero callers. Wire up or delete.
3. **Example-map example [8] claim not implemented:** "Esc on the board pane exits mux mode back to the single Board view" — in mux mode, Esc on the board pane is a no-op (BoardView Esc only clears a details strip selection, `board.rs:162`; the App-level Esc cascade at `events.rs:166-177` requires `active_view == ViewMode::Board`). No scenario covers it; `MuxExit` (the obvious carrier) is never emitted.
4. **Rule 12 ('m' toggles mux) has no scenario** in the feature file, though the behavior is implemented (`events.rs:155-158`) and tested (`mux001.rs:1501`).
5. **R1 scenario text vs live focus mismatch:** the scenario says "the split is 50/50 with the **agent pane focused**", but fresh entry deliberately focuses the **Board** pane (`mod.rs:160-170`, `dispatch_mux.rs:141-149` → `MuxFocus::Pane(0)`). The app-level test only asserts `cfg.focused_pane == 1` (persisted "home" focus), not the live focus. Fix the scenario text or the behavior.
6. **Lazy whole-file coverage links:** multiple scenarios link entire files as "implementation" (e.g. `mux_parser.rs:1..205`, `keys.rs:1..112`, `events.rs:1..300`, `layout.rs:1..222`, `mouse.rs:1..89`). They point at real code but carry no information; re-link to the specific functions.
7. **Test file hygiene:** `tests/mux001.rs` (1514 lines) lacks the mandatory leading `/** Feature: spec/features/rust-mux-mode.feature */` header comment, contains stale references to merged-away files (`mux001_app.rs`, `mux001_parser.rs`, `mux001_layout.rs` at lines 530, 558, 786, 985), and far exceeds the 300-line guideline.
8. **Multi-pane grids (3-4 panes) expose only ONE resizable divider:** `divider_rect` is computed only after pane 0 (`mod.rs:382-395`, `render.rs:60-73`); `adjust_split`/`split_to_min`/`split_to_max` only touch `splits[0]` (`mod.rs:274-299`); `set_pane_count` writes `splits[1..]` that nothing adjusts (`mod.rs:245`). Gaps between panes 2-3/3-4 are unpainted and unhit-testable. Acceptable as MVP (R4 says "the divider", singular) but needs a follow-up.

## 🟢 Observations (Nice to Have)

1. **Duplicate pane kinds are accepted:** `/mux board agent agent` and `/mux board board` parse fine (no duplicate check). Two `agent` panes would render the same `AgentView` twice — which is exactly the semantics the user's new expectation needs, so this is the seed of the follow-up work unit.
2. **DRY:** divider-rect computation duplicated between `MultiplexLayout::recompute_rects` (`mod.rs:382-395`) and `render.rs:60-73`; a shared `divider_rect_for(orientation, first_pane, body)` helper would remove it.
3. **`paint_divider`** computes a `vertical` bool and discards it (`render.rs:105-116`, `let _ = vertical;`).
4. **R10 divider assertion is weak:** `mux001.rs:523-526` asserts no row contains both `│` and `"MUX"` — nearly subsumed by the first assertion (no row contains `"MUX"` at all).
5. **Scenario naming:** "the mux files checkpoints command sets a two-pane list" (feature line 124) is awkward; "/mux files checkpoints sets a two-pane list" would match its siblings.
6. **Parser-level tests for `/mux 3/4/2`** (lines 616-653) only assert the parsed variant, with the "Then" step satisfied by comments — the dispatcher half is covered by the app-level tests, so acceptable, but the split across two functions per scenario makes @step auditing harder.
7. **No security or unbounded-growth concerns:** persistence is best-effort with `tracing` logging, no `unwrap`/`println`/`panic!` in production code, no TODO/FIXME/unimplemented, all loops bounded by pane count (≤4) or terminal size.

## Coverage Verification
- Feature file: `spec/features/rust-mux-mode.feature` — ISSUE: stale `spec/mux.json` references (lines 9, 41, 51-52) contradict rule 13; stale 52-col minimum in example-map rule [4]/example [3]; R1 "agent pane focused" vs live board focus; rule 12 has no scenario
- Test file(s): `rust/fspec-tui/tests/mux001.rs` — ISSUE: missing mandatory `Feature:` header comment; stale references to merged-away `mux001_{app,parser,layout}.rs`; otherwise @step comments match Gherkin step text exactly and assertions are substantive
- Impl file(s): `rust/fspec-tui/src/views/multiplex/{mod,layout,render,keys,mouse,presets}.rs`, `rust/fspec-tui/src/app/{dispatch_mux,mux_parser}.rs`, `rust/fspec-tui/src/store/mux_state.rs`, `rust/sessions/src/mux_config_persistence.rs` (+ `views/navigator.rs`, `app/{dispatch,events,state,bootstrap}.rs`) — ISSUE: stale `spec/mux.json` doc comments (mod.rs:61, mux_parser.rs:46); dead Action variants + `config_for_panes`; mod.rs over 300 lines; lazy whole-file coverage links
- Scenario coverage: 31/31 scenarios covered (fspec `show-coverage`: 100%); test line ranges verified to point at the correct test functions; impl line ranges point at real code but several are whole-file (see Warnings #6)
- Build/test: `cargo test -p codelet-fspec-tui --test mux001` → **52 passed, 0 failed**; `cargo check -p codelet-fspec-tui` → clean; `cargo clippy -p codelet-fspec-tui` → clean (only a pre-existing `too_many_arguments` warning in `codelet-core`, unrelated)

## Files Reviewed
- `spec/features/rust-mux-mode.feature`
- `rust/fspec-tui/tests/mux001.rs` (all 1514 lines, in chunks)
- `rust/fspec-tui/src/views/multiplex/mod.rs`
- `rust/fspec-tui/src/views/multiplex/layout.rs`
- `rust/fspec-tui/src/views/multiplex/render.rs`
- `rust/fspec-tui/src/views/multiplex/keys.rs`
- `rust/fspec-tui/src/views/multiplex/mouse.rs`
- `rust/fspec-tui/src/views/multiplex/presets.rs`
- `rust/fspec-tui/src/app/dispatch_mux.rs`
- `rust/fspec-tui/src/app/mux_parser.rs`
- `rust/fspec-tui/src/store/mux_state.rs`
- `rust/sessions/src/mux_config_persistence.rs`
- `rust/fspec-tui/src/views/navigator.rs` (mux sections)
- `rust/fspec-tui/src/app/events.rs`
- `rust/fspec-tui/src/app/dispatch.rs` (mux sync/auto-save section)
- `rust/fspec-tui/src/app/state.rs` (mux seams, via grep)
- `rust/fspec-tui/src/app/bootstrap.rs` (via grep)
- `rust/fspec-tui/src/views/board.rs` (key handling)
- `rust/fspec-tui/src/components/help_content.rs`
- `rust/fspec-tui/src/components/mod.rs` (Action variants, via grep)
- fspec work unit MUX-001 (example map, rules, assumptions, architecture notes) and `show-coverage rust-mux-mode` output
- Test/build outputs: `/tmp/mux001-test.txt`, `/tmp/mux001-check.txt`

---

## Fix Results (2026-08-26)

### MUX-001: Mux mode

- 🔴 Issue 1 (multiple agent panes / grouped agent-view cycling) → ✅ Tracked as **MUX-002** (child story, specifying): per-agent panes, grouped agent-view section, Shift+Right rotates agent views with non-agent panes pinned. Example map seeded (3 rules, 1 example).
- 🔴 Issue 2 (stale `spec/mux.json` references) → ✅ Fixed: feature file (doc string, example 8, Q/A), `mod.rs:61` doc, `mux_parser.rs` Save doc, `state.rs` docs, `bootstrap.rs` comment, work unit description + example-map example [7] all now say shared `fspec-config.json` under `tui.mux`.
- 🔴 Issue 3 (stale 52-col board minimum) → ✅ Fixed: example-map rule [4] removed and re-added as "R5 (corrected)" with 64 cols.
- 🟡 Issue 1 (mod.rs 404 lines) → ✅ Fixed: `recompute_rects` extracted to `views/multiplex/rects.rs`; mod.rs now 292 lines.
- 🟡 Issue 2 (dead Action variants + config_for_panes) → ✅ Fixed: `MuxToggle`, `MuxOn`, `MuxOff`, `MuxConfigApplied`, `MuxFocusPrev`, `MuxFocusNext`, `MuxSplitAdjust`, `MuxExit` deleted from the Action enum; only `MuxEnterWorkUnit` remains (the sole emitter). `config_for_panes` deleted.
- 🟡 Issue 3 (Esc-exits-mux claim) → ✅ Fixed per user directive: claim removed from example map + feature file; no Esc mux exit exists or is planned.
- 🟡 Issue 4 (rule 12 'm' toggle) → ✅ Resolved per user directive: the 'm' toggle was REMOVED entirely (user: only Shift+Left/Right keys + mouse + /mux subcommands). `handle_app_shortcut` arm deleted, `Action::MuxToggle` deleted, footer text updated, `/mux help` text updated.
- 🟡 Issue 5 (R1 agent-pane-focused vs live board focus) → ✅ Fixed per user decision: spec text updated to "Board pane focused on fresh entry (the agent pane is the persisted home focus)"; example-map rule re-added corrected.
- 🟡 Issue 6 (lazy whole-file coverage links) → ✅ Fixed: all 27 scenarios re-linked to specific function line ranges.
- 🟡 Issue 7 (test file hygiene) → ✅ Fixed: `Feature:` header comment added (as `//` comment after the crate-level `#![allow]`); test file now 1392 lines (still over 300 — pre-existing, flagged for follow-up).
- 🟡 Issue 8 (multi-pane single divider) → ⏸ Noted as follow-up (acceptable as MVP; R4 says "the divider" singular).
- 🟢 Observations 2-3 (DRY divider rect, unused `vertical` bool) → ✅ Fixed: `divider_rect_for` helper in `layout.rs` shared by render + recompute; `vertical` bool removed.

### Additional user directive (2026-08-26): remove ALL mux keybindings except Shift+Left/Right

- Tab pane/divider cycling removed: `KeyDecision::TabNext`, `MultiplexLayout::tab_next`, `DividerKey` enum, keyboard divider resize (`adjust_split`, `split_to_min`, `split_to_max`), `MuxFocus::Divider` variant (focus is now a plain pane index).
- 4 scenarios removed from the feature file (focused-divider Right/Home/End/Esc) + their 4 tests removed; coverage re-linked (27/27 scenarios, 100%).
- Mouse divider drag RETAINED (explicit user instruction).
- Tab is now reserved for the agent view's turn-select mode inside the focused agent pane.

## Final Verification
- All tests pass: ✅ (mux001: 47 passed; codelet-fspec-tui --lib: 508 passed; codelet-sessions --lib: 57 passed)
- Build succeeds: ✅ (cargo check clean; cargo clippy clean — only pre-existing codelet-core `too_many_arguments` warning)
- Coverage complete: ✅ (27/27 scenarios, 100%, specific line ranges)
- Feature files valid: ✅ (fspec validate)
- Tags valid: ✅ (unchanged)
