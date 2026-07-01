# Review: RPC-397 — View-specific accurate help content for board and agent

**Date:** 2026-07-01
**Reviewer:** Claude Code review skill + subordinate reviewer agent
**Status: PASS** (0 critical; 2 accuracy warnings FIXED; 1 tag warning = false alarm)

## 🔴 Critical: None

## 🟡 Warnings
1. **"? Toggle this help" overstated** — `?` opens the dialog; only ESC closes it.
   → **FIXED**: changed to "? Show this help".
2. **"ESC Exit fspec" imprecise** — ESC opens the "Exit fspec?" confirmation dialog (RPC-102),
   it does not exit immediately.
   → **FIXED**: changed to "ESC Exit (confirm)".
3. **Feature-group tag** — reviewer flagged missing feature-group tag. **False alarm**: the feature
   carries `@help-text` AND `@dialog` (both registered feature-group tags) plus `@tui` (component)
   and `@RPC-397` (work-unit). Requirement satisfied.

## 🟢 Observations
- Agent slash-command section is derived from `SLASH_COMMANDS` (help_content.rs:78-80) — zero
  hardcoding; all 17 commands + real descriptions; cannot drift. Excellent.
- `new()`/`default()` delegate to `for_board()` — sensible back-compat.
- RPC-027 checks NOT weakened: cyan border, bold "Help" inner title, title-absent-from-border,
  `render_dialog` import all still asserted.
- help_dialog.rs = 281 LoC, help_content.rs = 116 LoC (both < 300).
- No unwrap/todo/unimplemented/panic in production paths.
- Snapshots regenerated for the new board content; no stale/pending snapshots.
- Board & agent keybindings cross-checked against views/board.rs and views/agent/dispatch.rs —
  accurate (after the two wording fixes).

## Coverage Verification
- Feature: 3 scenarios, valid G/W/T, no placeholders, tags `@RPC-397 @tui @dialog @help-text`, doc string present.
- Example Map: 5 rules + 3 examples → 3 scenarios; no unanswered questions; arch notes match impl.
- Tests: 3/3 scenarios; @step comments word-for-word; real assertions (board hints + no slash;
  agent hints + slash list w/ descriptions; neither has "q Quit"; both have Ctrl+D+Quit).
- Scenario coverage: 100% (3/3), audit 9/9 files valid.
- Build/Test: full suite 2005 passed / 0 failed; clippy 0 warnings; fmt clean.

## Files Reviewed
- spec/features/view-specific-accurate-help-content-for-board-and-agent.feature
- codelet/fspec-tui/tests/help_dialog_content_rpc397.rs
- codelet/fspec-tui/src/components/help_content.rs
- codelet/fspec-tui/src/components/help_dialog.rs
- codelet/fspec-tui/src/app/events.rs, src/app/dispatch_slash_commands.rs
- codelet/fspec-tui/src/views/agent/slash_commands.rs (source of truth)
- codelet/fspec-tui/src/views/board.rs, src/views/agent/dispatch.rs (accuracy cross-check)
- codelet/fspec-tui/tests/rpc027_dialog_parity_bcd.rs
