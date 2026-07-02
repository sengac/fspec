# Review: RPC-402 — Shift+Enter newline unreachable in agent input — keyboard enhancement flags never enabled

**Date:** 2026-07-01
**Reviewer:** ACDD compliance reviewer (automated)

## Status: WARN

## 🔴 Critical Issues (Must Fix)

1. **Rule [2] (Press-only filter) is NOT wired into the real production event path.** The `KeyEventKind::Press` filter lives only in `MultiLineInput::handle_event_gated` (`codelet/fspec-tui/src/views/agent/multiline_input.rs:235-241`), but the actual AgentView dispatch path bypasses it: `codelet/fspec-tui/src/views/agent/dispatch.rs:247-250` routes `Event::Key(key)` directly to `self.input.handle_key_gated(key.code, key.modifiers, gate)`, discarding `key.kind` entirely. Nothing upstream filters kind on this path either — `app/events.rs:53-104` Stage 3 (Navigator → AgentView) has no kind check (the `key.kind != KeyEventKind::Release` guards at `app/events.rs:95` and `:110` cover only Stage-4 app shortcuts and the DisconnectDialog). Consequence: a Release/Repeat Shift+Enter or plain Enter delivered by the terminal (crossterm delivers Release events on Windows unconditionally; Repeat under fuller kitty protocols) reaches `handle_enter` and can double-submit or double-insert. The passing test for scenario "Key release events are ignored by the input" exercises `handle_event(&release)` — the widget API — which is **not** the path `dispatch.rs` uses, so the test does not pin the real behavior ("WHO CALLS THIS?" violation). Architecture note [2] explicitly said "Filter KeyEventKind::Press at the input boundary (multiline_input handle_key_gated or dispatch.rs)" — it landed in neither of the paths dispatch.rs actually exercises for key events. Fix: either check `key.kind != KeyEventKind::Press → ignore` in `dispatch.rs` before line 248, or route the whole `Event::Key` through `handle_event_gated`. (The Esc/Ctrl+C/PageUp/Tab/Shift-arrow branches in `dispatch.rs:167-228` are similarly kind-unfiltered.)

## 🟡 Warnings (Should Fix)

1. **Teardown is fail-fast, not best-effort, contradicting the feature architecture doc string.** `restore_terminal_modes` (`terminal.rs:209-216`) uses `?` per command: if e.g. `DisableBracketedPaste` errors, the loop aborts — `PopKeyboardEnhancement`, `LeaveAlternateScreen`, and `DisableRawMode` are all skipped, and because `ENHANCEMENT_PUSHED` was already `swap`ped to `false` (line 210), a second teardown (Drop after panic-hook restore) will never retry the pop. The terminal-keyboard-enhancement-flags.feature doc string says "teardown (incl. Drop/panic restore) must be best-effort". Each teardown command should be attempted independently (`let _ = execute_mode_command(cmd)` or collect errors).
2. **Ungated Ctrl+Enter fallthrough still inserts a newline, including while Compacting.** `handle_enter` returns `None` for other modifier combos (`multiline_input_enter.rs:52-54`), and `is_edit_keystroke` (`:62-67`) does not include `KeyCode::Enter`, so Ctrl+Enter falls to `textarea.input()` (`multiline_input.rs:217-220`) where tui-textarea inserts a newline — bypassing `gate.block_edits`. RPC-402's stated goal was "closing the accidental tui-textarea fallthrough"; it was closed only for Shift/Alt. Either route all modifier-Enter combos through the gated newline branch or add Enter to `is_edit_keystroke`.
3. **Placeholder hint does not fit 80-column rendering.** `INPUT_PLACEHOLDER_HINT` (`views/agent.rs:76-77`) is 106 display chars; the input body at 80 cols is ~76 cols (border + `"> "` prompt), and `render_with_prompt` (`multiline_input.rs:290-293`) paints it via `Paragraph` without wrap — the newly added `'Shift+Enter' newline` segment at the END of the string is exactly the part truncated on any terminal narrower than ~112 cols. Consider putting the new hint first or shortening the string.
4. **Coverage impl line-range drift for scenario "Key release events are ignored by the input".** Coverage points at `multiline_input.rs:222-230`, which spans a blank line, doc comments, and the ungated `handle_event` wrapper; the actual Press-kind filter is at lines 235-241 (`handle_event_gated`). Re-link to 235-246.
5. **Stale feature-file architecture line references.** agent-input-multiline-newline-keys.feature doc string (lines 10-11) cites "Shift+Enter branch already exists at :174-182; keep plain-Enter submit at :165-173" — those branches now live in `multiline_input_enter.rs:35-51`, not `multiline_input.rs`. Living documentation should be updated (same for work-unit architecture note [1]).

## 🟢 Observations (Nice to Have)

1. Stale "red phase" commentary in both test files: `terminal_keyboard_enhancement_rpc402.rs:37-40` still says "This file FAILS TO COMPILE until the seam lands", and `agent_input_multiline_newline_rpc402.rs` header says "(red phase)". Harmless but misleading now that the implementation is done.
2. Feature 1's `Feature:` title is bug-worded ("Shift+Enter newline unreachable … never enabled") rather than capability-worded; the file NAME (`agent-input-multiline-newline-keys.feature`) is correctly capability-based, so this is cosmetic.
3. Rule [3] ("grows one row per logical line up to 6 visible rows") has no scenario asserting the 6-row cap itself — scenarios only assert 1/2/3 rows. The cap is pinned by pre-existing RPC-019 tests (`visible_rows()` clamp), so acceptable, but the rule→scenario chain is incomplete within this feature.
4. If `TerminalGuard::init` fails partway (e.g. `EnterAlternateScreen` errors after a successful flag push), no guard object exists yet and nothing restores unless a panic occurs — pre-existing behavior (raw mode leaked the same way before RPC-402), but worth noting; the ENHANCEMENT_PUSHED flag would remain set with no consumer.
5. The supplementary `alt_enter_is_gated_while_compacting` test deliberately has no @step comments and documents why (augments rather than maps 1:1 to a scenario) — good practice.
6. `ENHANCEMENT_PUSHED` double-pop analysis: **sound**. The `swap(false, SeqCst)` in `restore_terminal_modes` (`terminal.rs:210`) is atomic, so panic-hook restore followed by `TerminalGuard::Drop` cannot double-pop — the second call sees `pushed=false` and plans no pop. Ordering `SeqCst` is stronger than needed but harmless. One theoretical wrinkle: it is process-wide, so two concurrent `TerminalGuard`s would share one flag — but the app only ever creates one guard (`app/events.rs:192`), so this is fine in practice.
7. `terminal_mode_plan` reuse is genuine, not a parallel copy: `enable_terminal_modes` (`terminal.rs:185-204`) and `restore_terminal_modes` (`terminal.rs:209-216`) both iterate the plan the tests assert on. The `EnableRawMode => {}` skip inside the setup loop (raw mode enabled eagerly before the support query) is a small wart — the plan's raw-mode slot is decorative on the execution side — but it is documented inline and preserves the test's ordering assertion.
8. Alt+Enter gate check is correct: the branch tests `gate.block_edits` (`multiline_input_enter.rs:46`) and plain Enter tests `gate.suppress_enter` (`:37`) — both gates set together by dispatch.rs during Compacting (`dispatch.rs:243-246`), and the supplementary test pins the Alt path.
9. Help content change is consistent: `help_content.rs:57-58` adds "Shift+Enter Newline" and "Alt+Enter Newline (legacy terminals)" to `agent_help_lines()`, consumed by `HelpDialog::for_agent()` (`help_dialog.rs:73-74`), which is reachable via `/help` (`dispatch_slash_commands.rs:33`). Board help correctly unchanged.
10. Minor duplication: `role_dialog.rs:133-153` has its own Enter/Shift+Enter textarea handling that intentionally diverges (documented inline as matching MultiLineInput behavior without sharing code). Acceptable for a small dialog, but a future shared enter-routing helper could DRY this.

## Build & Test Verification

- `cargo test -p codelet-fspec-tui --test agent_input_multiline_newline_rpc402 --test terminal_keyboard_enhancement_rpc402`: **8 passed, 0 failed** (6 + 2). Log: /tmp/review402_tests.log
- `cargo clippy -p codelet-fspec-tui --all-targets`: **clean** (0 warnings, 0 errors). Log: /tmp/review402_clippy.log
- File sizes (300-LoC ceiling): terminal.rs 236 ✅, multiline_input.rs 297 ✅, multiline_input_enter.rs 67 ✅, views/agent.rs 299 ✅, help_content.rs 117 ✅
- No `unwrap()`/`expect()` in reviewed production files (test files use them under explicit `#![allow]`, which is acceptable). No TODO/FIXME/HACK/todo!/unimplemented! anywhere in `fspec-tui/src`.

## Example Map Alignment

- Rules [0]–[4] each map to scenarios (rule 0 → both terminal-flags scenarios; rule 1 → Shift/Alt/plain-Enter scenarios; rule 2 → Release scenario; rule 3 → row-count assertions, partially — see Observation 3; rule 4 → compacting scenario + supplementary Alt test).
- Examples [0]–[6] all map to scenarios across the two feature files. No unanswered questions. Architecture notes match the implementation EXCEPT note [2] (Press filter placement — see Critical 1) and the now-stale line numbers in note [1] (see Warning 5).

## Coverage Verification

- Feature file: spec/features/agent-input-multiline-newline-keys.feature — OK (valid Gherkin, Given/When/Then ordered, @RPC-402 tag, architecture doc string present; stale line refs per Warning 5)
- Feature file: spec/features/terminal-keyboard-enhancement-flags.feature — OK (valid Gherkin, @RPC-402 tag, architecture doc string present)
- Test file(s):
  - codelet/fspec-tui/tests/agent_input_multiline_newline_rpc402.rs — OK (@step comments EXACTLY match all 5 scenarios' step text; real behavioral assertions; feature header comment present)
  - codelet/fspec-tui/tests/terminal_keyboard_enhancement_rpc402.rs — OK (@step comments exactly match both scenarios; asserts real plan ordering; feature header present; stale red-phase commentary per Observation 1)
- Impl file(s):
  - codelet/fspec-tui/src/terminal.rs — OK (coverage line ranges 90-113 and 185-217 verified against actual `terminal_mode_plan` / `enable_terminal_modes`+`restore_terminal_modes` code)
  - codelet/fspec-tui/src/views/agent/multiline_input_enter.rs — OK (ranges 26-55 / 35-51 verified)
  - codelet/fspec-tui/src/views/agent/multiline_input.rs — ISSUE (Release-scenario impl range 222-230 is off; real filter at 235-241 — Warning 4; and the filter is bypassed by dispatch.rs — Critical 1)
  - codelet/fspec-tui/src/views/agent.rs (INPUT_PLACEHOLDER_HINT) — ISSUE (hint truncates at 80 cols — Warning 3)
  - codelet/fspec-tui/src/components/help_content.rs — OK
- Scenario coverage: 7/7 scenarios covered (5/5 agent-input + 2/2 terminal-flags), 100% per fspec show-coverage; test line ranges verified accurate for all 7.

## Files Reviewed

- spec/features/agent-input-multiline-newline-keys.feature
- spec/features/terminal-keyboard-enhancement-flags.feature
- spec/attachments/RPC-402/investigation.md
- codelet/fspec-tui/tests/agent_input_multiline_newline_rpc402.rs
- codelet/fspec-tui/tests/terminal_keyboard_enhancement_rpc402.rs
- codelet/fspec-tui/src/terminal.rs
- codelet/fspec-tui/src/views/agent/multiline_input.rs
- codelet/fspec-tui/src/views/agent/multiline_input_enter.rs
- codelet/fspec-tui/src/views/agent/multiline_input_paste.rs
- codelet/fspec-tui/src/views/agent/dispatch.rs
- codelet/fspec-tui/src/views/agent.rs
- codelet/fspec-tui/src/components/help_content.rs
- codelet/fspec-tui/src/components/role_dialog.rs (partial — enter-handling duplication check)
- codelet/fspec-tui/src/app/events.rs
- fspec: show-work-unit RPC-402, show-coverage (both features)

## Fix Results (2026-07-01, post-review remediation)

- 🔴 Critical 1 (Press filter not on real dispatch path) → ✅ Fixed: dispatch.rs Event::Key arm drops `key.kind != KeyEventKind::Press` before ANY branch (dispatch.rs:71-80); input fallthrough now routes through `handle_event_gated`; widget filter kept as defense-in-depth. dispatch.rs split (popup helpers → new dispatch_popups.rs, 114 LoC; dispatch.rs now 213 LoC). Scenario test strengthened to drive the REAL path (App::handle_event → Navigator → AgentView dispatch) asserting buffer unchanged + no InputSubmitted action; widget-boundary check retained as supplementary test.
- 🟡 Warning 1 (fail-fast teardown) → ✅ Fixed: restore_terminal_modes attempts every teardown command independently, records first error, returns after all ran (terminal.rs:215-228).
- 🟡 Warning 2 (ungated Ctrl+Enter) → ✅ Fixed: handle_enter treats ANY non-empty-modifier Enter as gated newline (multiline_input_enter.rs:29-54); Enter can no longer reach textarea.input(). Supplementary tests: ctrl_enter_inserts_a_newline_instead_of_submitting, ctrl_enter_is_gated_while_compacting.
- 🟡 Warning 3 (placeholder hint truncated at 80 cols) → ✅ Fixed: INPUT_PLACEHOLDER_HINT now 95 chars with 'Shift+Enter' newline first (ends at char 39); rpc095 exact assertion + rpc019 step/assertion updated in sync; 2 insta snapshots re-recorded after diff review.
- 🟡 Warning 4 (coverage drift) → ✅ Fixed: Release scenario re-linked to dispatch.rs:71-80; all shifted test ranges re-linked; audit-coverage passes.
- 🟡 Warning 5 (stale doc string) → ✅ Fixed: feature doc string rewritten to reference multiline_input_enter.rs / dispatch.rs Press filter / best-effort teardown.
- 🟢 Observation 1 (stale red-phase comments) → ✅ Removed from both test headers.

## Final Verification
- Full crate: cargo test -p codelet-fspec-tui → 2049 passed / 0 failed
- clippy --all-targets: clean; cargo fmt --check: clean
- Coverage: 5/5 + 2/2 scenarios, audit-coverage all mappings valid
- Feature files valid; tags valid
