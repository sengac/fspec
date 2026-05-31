# Review: RPC-063 — /role slash command end-to-end (UI dialog)

**Date:** 2026-05-25
**Reviewer:** Claude Code (fspec review skill)
**Scope:** RPC-063 only (no scope creep)

## Status: ✅ PASS (after fixes applied)

---

## Discovery

- **Parent:** RPC-030 (epic)
- **Depends on:** RPC-062
- **Children:** None (leaf story)
- **Type:** story
- **Status at review start:** done

### Files Reviewed

**Feature files:**
- `spec/features/role-dialog-component.feature` (9 scenarios, all covered)
- `spec/features/role-slash-command-end-to-end-ui-dialog.feature` (8 scenarios, all covered)

**Implementation:**
- `codelet/fspec-tui/src/components/role_dialog.rs` (212 lines — under 300 LoC ceiling)
- `codelet/fspec-tui/src/app/dispatch_rpc063.rs` (45 lines — well under ceiling)
- `codelet/fspec-tui/src/app/slash_parser.rs` (slash routing, OpenRoleDialog variant)
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (submit-line + palette wiring)

**Tests:**
- `codelet/fspec-tui/tests/role_dialog_rpc063.rs` (9 tests, all passing)
- `codelet/fspec-tui/tests/slash_role_rpc063.rs` (8 tests, all passing)
- `codelet/fspec-tui/tests/slash_command_wiring_rpc022.rs` (10 tests, updated for RPC-063 behavior)
- `codelet/fspec-tui/tests/app_dispatch_rpc022.rs` (9 tests, still valid)

**Attachments:**
- `spec/attachments/RPC-063/slash-role.md`
- `spec/attachments/RPC-063/ast-research-role-dialog.md`

---

## 🔴 Critical Issues (Found and Fixed)

### 1. Clippy `uninlined_format_args` violation in role_dialog.rs

The workspace enforces `uninlined_format_args = "deny"`. The original code used a named format argument (`sep = FOOTER_SEPARATOR`) which clippy rejects.

**Original (role_dialog.rs:163-166):**
```rust
let footer = format!(
    "Enter Save{sep}Ctrl+D Clear{sep}Esc Cancel",
    sep = FOOTER_SEPARATOR
);
```

**Fixed:**
```rust
let sep = FOOTER_SEPARATOR;
let footer = format!("Enter Save{sep}Ctrl+D Clear{sep}Esc Cancel");
```

Result: ✅ Clippy passes for `role_dialog.rs` and `dispatch_rpc063.rs` (with `--no-deps`).

---

## 🟡 Warnings (Found and Fixed)

### 2. Misleading test function names in slash_command_wiring_rpc022.rs

Per architecture note [I], tests asserting the old "bare /role clears" semantics in `slash_command_wiring_rpc022.rs` MUST be updated to assert the new "opens dialog" semantics. The test BODIES were updated correctly, but the function NAMES still claimed the old behavior — a direct contradiction between name and assertion.

**Renamed:**
- `submitting_bare_slash_role_is_treated_as_a_clear` → `submitting_bare_slash_role_opens_the_role_dialog`
- `slash_popup_selection_of_role_is_treated_as_a_clear_with_no_notice` → `slash_popup_selection_of_role_opens_the_role_dialog`

Both tests' bodies already asserted RPC-063's new behavior (the `assert!(app.compositor().contains(ROLE_DIALOG_ID))` + "must NOT clear" assertions). Only the names lied. Now the names match the assertions.

---

## 🟢 Observations (Out of Scope — Not Fixed)

### Pre-existing workspace clippy violations (NOT introduced by RPC-063)

Running `cargo clippy -p codelet-fspec-tui --tests` surfaces errors in unrelated files from earlier RPC cards:
- `core/src/scheduler/agent_job.rs`, `cron_utils.rs`, `engine.rs`, `shell_job.rs` (RPC-058)
- `fspec-tui/src/app/dispatch_rpc050.rs`, `dispatch_rpc054.rs`, `dispatch_rpc055.rs`, `dispatch_rpc057.rs`, `dispatch_rpc059.rs`
- `fspec-tui/src/app/loop_parser.rs`
- `fspec-tui/src/views/blocklist/mod.rs`, `provider_settings/mod.rs`

These are NOT in scope for RPC-063 and predate this work unit. Belong to the originating RPC cards.

### Stale documentation in rpc022-slash-command-wiring.feature

The RPC-022 feature file still documents the OLD "bare /role clears" semantics:
- Doc string line 22 (`/role → SetSessionRole(sid, None)`)
- Scenario Outline line 49 (`/role → ClearRole`)
- Scenario line 103 ("Submitting bare '/role' is treated as a clear")
- Scenario line 136 ("Slash popup selection of /role is treated as a clear")

This is RPC-022's spec artifact. The architecture note [I] in RPC-063 only mandates updating the TEST files, not the prior feature file. Updating the rpc022-slash-command-wiring.feature would technically be scope creep into RPC-022's domain. RPC-063 already documents the NEW behavior in its own feature files (`role-slash-command-end-to-end-ui-dialog.feature`).

**Recommendation:** A future cleanup card could either delete the stale scenarios from rpc022-slash-command-wiring.feature or update them with a "[superseded by RPC-063]" annotation. Not in this card's scope.

---

## Verification

### Rules → Scenarios → Tests Traceability

All 12 rules in the example map (`show-work-unit RPC-063`) are covered by scenarios in the feature files and assertions in the tests:

| Rule | Covered by |
|------|-----------|
| [0] Bare /role opens dialog seeded from current role | role-slash-command-end-to-end-ui-dialog.feature scenarios (palette + submit-line) |
| [1] /role <text> still sets directly | "Submitting /role You are a code reviewer" scenario |
| [2] /role clear still clears directly | "Submitting /role clear" scenario |
| [3] Enter dispatches SetSessionRole(Some) | role-dialog-component.feature "Enter saves" scenario |
| [4] Ctrl+D dispatches SetSessionRole(None) | "Ctrl+D clears the role" scenario |
| [5] Esc cancels with no Action | "Esc cancels the dialog" scenario |
| [6] Priority::Foreground, Accent::Cyan, title "Role" | "RoleDialog renders" scenario |
| [7] /role from palette with no session is no-op | "Palette pick of /role with no active session" scenario |
| [8] Seeds from AgentViewStore::role_for | "Palette pick of /role on a session with an existing role" scenario |
| [9] id == "role-dialog" | ROLE_DIALOG_ID constant + render scenario |
| [10] Opening is idempotent | "Opening the RoleDialog is idempotent" scenario |
| [11] Footer reads "Enter Save │ Ctrl+D Clear │ Esc Cancel" | render scenario asserts each substring |

### @step Comments

All scenarios in both feature files have corresponding `// @step` comments in their test files that match the Gherkin step text verbatim.

### Coverage

- `role-dialog-component.feature`: 100% (9/9 scenarios, FULLY COVERED)
- `role-slash-command-end-to-end-ui-dialog.feature`: 100% (8/8 scenarios, FULLY COVERED)

### Implementation Quality

- ✅ Files under 300 LoC: `role_dialog.rs` (212), `dispatch_rpc063.rs` (45)
- ✅ No `unwrap()`/`expect()`/`panic!()` in production code (only in `#[cfg(test)]`)
- ✅ Proper error handling with `if let Some(...) else { return; }` early-return pattern
- ✅ No TODO/FIXME/HACK markers
- ✅ End-to-end wiring: SlashCommandAction::Role → handle_open_role_dialog → RoleDialog → Action::SetSessionRole → existing handle_set_session_role
- ✅ Reuses existing `Action::SetSessionRole` variant (no new variant required, per architecture note [C])
- ✅ Idempotency check via `compositor.contains(ROLE_DIALOG_ID)` (architecture note [E])
- ✅ No backend.get_session_role round-trip (architecture note [E] — seeds from AgentViewStore directly)

---

## Fix Results

### RPC-063: /role slash command end-to-end (UI dialog)
- 🔴 Issue 1 (clippy uninlined_format_args in role_dialog.rs) → ✅ Fixed: bound `let sep = FOOTER_SEPARATOR;` before the format string so the captured identifier works
- 🟡 Issue 2 (misleading test names in slash_command_wiring_rpc022.rs) → ✅ Fixed: renamed `submitting_bare_slash_role_is_treated_as_a_clear` → `_opens_the_role_dialog`, and `slash_popup_selection_of_role_is_treated_as_a_clear_with_no_notice` → `_opens_the_role_dialog`

### Final Verification
- All RPC-063 tests pass: ✅ (9 + 8 = 17 tests)
- All related RPC-022 tests pass: ✅ (10 + 9 = 19 tests)
- Build succeeds: ✅ `cargo build -p codelet-fspec-tui`
- Clippy clean on RPC-063 files: ✅ (no warnings/errors on role_dialog.rs, dispatch_rpc063.rs, slash_role_rpc063.rs, slash_command_wiring_rpc022.rs)
- Feature files valid: ✅ both pass `fspec validate`
- Coverage: ✅ 100% (17/17 scenarios across both feature files)

---

## Summary Table

| Work Unit | Title                                    | Status | Issues   |
|-----------|------------------------------------------|--------|----------|
| RPC-063   | /role slash command end-to-end (UI dialog) | ✅ PASS | 2 fixed  |
