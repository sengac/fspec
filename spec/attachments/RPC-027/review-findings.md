# Epic Review: RPC-027 — Refactor all Rust RPC dialogs to match TypeScript Ink dialog theming

**Date:** 2026-05-19
**Reviewer:** Claude Code (`@spec/skills/review-skill.md`)
**Work Units Reviewed:** 1 (RPC-027 — single story, 5 feature files, no children)

## Summary
- 🔴 Critical: 1 issue (action wiring gap)
- 🟡 Warnings: 7 issues (stale headers, @step text mismatches, weak tests)
- 🟢 Observations: many — see per-feature reports

## Methodology
1. Spawned 5 subordinate ACDD-reviewer agents in parallel via `AgentManager`.
2. Each agent reviewed one of the 5 RPC-027 feature files + its test + its impl.
3. Collected findings, closed agents, consolidated below.
4. Applied all fixes sequentially in the supervisor session.

## Findings by Feature File

### 1. rpc027-dialog-theme.feature — **WARN → FIXED**
- 🟡 Test file header referenced **non-existent** `spec/features/rpc027-dialog-theme-parity.feature` (working title).
- 🟡 Stale "RED phase" comment in `tests/rpc027_dialog_theme.rs` claimed the module contained `unimplemented!()` — long obsolete.
- 🟢 Gherkin `Given/Then/Given/Then` smell in two `label_description_row` scenarios — unconventional but valid. Left as-is.

### 2. rpc027-help-disconnect-thinking-dialogs.feature — **WARN → FIXED**
- 🔴 **CRITICAL WIRING GAP**: `Action::SetThinkingLevelDefault` was emitted by `thinking_level_dialog.rs:146` on pressing `D`, but **never dispatched**. `try_dispatch_rpc022` had no match arm and the action was silently dropped. Architecture note [4] explicitly required a handler in `dispatch_rpc022.rs`. Doc comment in `components/mod.rs:373` was a **lie** — claimed `App::dispatch spawns backend.set_thinking_level_default(...)` while no such call site existed.
- 🟡 Source-shape test only verified trait declarations; did not pin the dispatch wiring.
- 🟡 Test file header referenced non-existent `rpc027-dialog-theme-parity.feature`.

### 3. rpc027-model-confirm-dialogs.feature — **WARN → FIXED**
- 🟡 `@step` text mismatch at `tests/rpc027_dialog_parity_ef.rs:327` — missing trailing word **"modifier"** vs. feature step.
- 🟡 Test file header referenced non-existent `rpc027-dialog-theme-parity.feature`.

### 4. rpc027-slash-file-popups.feature — **PASS (minor warnings) → FIXED**
- 🟡 Test file header referenced non-existent `rpc027-dialog-theme-parity.feature`.
- 🟢 `slash_command_popup.rs` (298) and `file_search_popup.rs` (294) at the edge of the 300-LoC ceiling. No fix needed but flagged for future additions.

### 5. rpc027-refactor-invariants.feature — **WARN → FIXED**
- 🟡 `@step` text mismatch at `tests/rpc027_dialog_parity_ij.rs:99` — paraphrased step text instead of verbatim match.
- 🟡 Test file header referenced non-existent `rpc027-dialog-theme-parity.feature`.
- 🟢 TS-unmodified test silently early-returns if `git` is unavailable. Behaviour is acceptable in CI environments with git always present.

## Fix Results

### Wiring fix (Critical)
- Added `App::handle_set_thinking_level_default(session_id, level)` in `src/app/dispatch_rpc022.rs` mirroring the `handle_thinking_level_selected` pattern: spawns a fire-and-forget `tokio` task that calls `backend.set_thinking_level_default(session_id, level)`.
- Added `Action::SetThinkingLevelDefault(s, l)` arm to `try_dispatch_rpc022` so `App::dispatch` actually routes the action.
- Extended `set_thinking_level_default_is_wired_through_the_backend_trait_stack` test with two source-shape assertions:
  - `dispatch_rpc022.rs` contains `Action::SetThinkingLevelDefault`
  - `dispatch_rpc022.rs` contains `backend.set_thinking_level_default(`
- Added Gherkin step `Then dispatch_rpc022.rs routes Action::SetThinkingLevelDefault to backend.set_thinking_level_default` to pin the new assertion.

### File-size hygiene
- Adding the handler pushed `dispatch_rpc022.rs` from 299 → 324 LoC (over the 300 ceiling per CLAUDE.md).
- Refactor: extracted `parse_slash_command` + `SlashCommandParse` + their unit test into a new sibling module `src/app/slash_parser.rs` (95 LoC).
- Updated `app/mod.rs` to re-export from `slash_parser` (backwards-compatible — `lib.rs` and external callers still use `App::parse_slash_command`).
- Updated `src/app/dispatch_rpc020.rs` import to point at `super::slash_parser`.
- Final sizes: `dispatch_rpc022.rs` = 236 LoC ✅, `slash_parser.rs` = 95 LoC ✅.

### Header / comment hygiene
- All 5 RPC-027 test files now reference their actual feature file paths (no more `rpc027-dialog-theme-parity.feature`).
- Removed obsolete "RED phase" / "unimplemented!() bodies" header text from `rpc027_dialog_theme.rs`.

### @step text matching
- `tests/rpc027_dialog_parity_ef.rs:327` — appended `modifier` so the `@step` line matches the Gherkin verbatim.
- `tests/rpc027_dialog_parity_ij.rs:99` — replaced `"a bare \"Popup::new(\" import from tui_popup"` with the verbatim feature step `"the substring \"Popup::new(\""`.

## Final Verification
- `cargo build` (codelet/fspec-tui): ✅ passes
- `cargo test --tests` (codelet/fspec-tui): ✅ **all 76 test groups pass, 0 failures** (full suite, not just RPC-027)
- All 5 RPC-027 feature files: ✅ 100% coverage (11+11+8+8+6 = 44 scenarios linked)
- `fspec validate spec/features/rpc027-*.feature`: ✅ valid
- All dialog source files under 300 LoC ceiling:
  - dialog_theme.rs = 294
  - dialog_theme_rows.rs = 39
  - help_dialog.rs = 157
  - disconnect_dialog.rs = 204
  - thinking_level_dialog.rs = 208
  - model_selector_dialog.rs = 272
  - model_selector_dialog_rows.rs = 73
  - confirm_dialog.rs = 246
  - slash_command_popup.rs = 298
  - file_search_popup.rs = 294
  - dispatch_rpc022.rs = 236 (was 324 pre-refactor)
  - slash_parser.rs = 95 (new)

## Summary Table

| Work Unit | Title | Status | Issues |
|-----------|-------|--------|--------|
| RPC-027 | Refactor all Rust RPC dialogs to match TypeScript Ink dialog theming | ✅ PASS | 1 critical + 7 warnings fixed |
