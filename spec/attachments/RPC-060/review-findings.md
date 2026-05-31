# Review: RPC-060 — Isolated session creation + AgentView /new isolated flow

**Date:** 2026-05-24
**Reviewer:** Claude Code (fspec review skill)
**Status (initial):** WARN
**Status (final):** PASS

This card has **no children** — it is a leaf story under RPC-030. Review covers just RPC-060.

---

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 1
- 🟢 Observations: 3

---

## 🔴 Critical Issues (Must Fix)

None.

---

## 🟡 Warnings (Should Fix)

### W1. "And the dialog accent is cyan" @step does not actually verify the accent

**Location:** `codelet/fspec-tui/tests/isolated_session_dialog_rpc060.rs:115-117`

```rust
// @step And the dialog accent is cyan
assert_eq!(dialog.priority(), Priority::Foreground);
assert_eq!(dialog.id(), CREATE_SESSION_DIALOG_ID);
```

The Gherkin step asserts the dialog accent is *cyan*, but the test verifies the
`priority` and `id` of the dialog instead — neither of which has anything to do
with the accent color. The implementation does set `Accent::Cyan` correctly
(`create_session_dialog.rs:236`), but the test does not actually exercise that
contract. This is a real test-to-spec drift.

**Fix:** Expose a public accessor `accent(&self) -> Accent` (backed by a single
module-level constant so the render path and the accessor cannot diverge) and
assert it in the test.

---

## 🟢 Observations (Nice to Have)

### O1. /isolation slash command test doesn't directly verify `preselect=Some(Isolated)`

**Location:** `codelet/fspec-tui/tests/isolated_session_dialog_rpc060.rs:309-332`

The scenario step "Then a CreateSessionDialog is pushed onto the compositor at
Priority::Foreground with preselect=Some(Isolated)" only asserts the dialog is
mounted on the compositor; it does not inspect the dialog's selected option.

**Mitigation:** Cross-covered by:
1. `create_session_dialog_accepts_isolated_preselection` — proves
   `CreateSessionDialog::new(Some(Isolated), None)` ends up selected on
   `Isolated`.
2. `dispatch_rpc020_routes_isolation_to_open_create_session_dialog` (source
   shape) — proves `dispatch_rpc020.rs` sends
   `OpenCreateSessionDialog { preselect: Some(CreateSessionOption::Isolated) }`.

Leave as-is; the chain is covered.

### O2. Coverage line ranges drift by 1–3 lines for several scenarios

For example:
- Scenario 10: coverage 307–330, actual fn 311–332
- Scenario 11: coverage 332–373, actual fn 336–374

The ranges still point at the right test, so navigation works; this is purely
cosmetic.

### O3. Mouse handling (`ScrollLeft/Up/Right/Down`) is implemented but not exercised

`create_session_dialog.rs:179-191` handles mouse scroll events but no scenario
covers it. This is bonus functionality consistent with sibling dialogs; leaving
it untested matches the rest of the suite.

---

## Coverage Verification

- Feature file: `spec/features/rpc060-isolated-session-dialog.feature` — OK (validates clean)
- Test file: `codelet/fspec-tui/tests/isolated_session_dialog_rpc060.rs` — OK (16 tests, all pass; 60 @step comments, 1:1 with 60 Gherkin steps)
- Source-shape test: `codelet/fspec-tui/tests/source_shape_rpc060.rs` — OK (6 tests, all pass)
- Impl files:
  - `codelet/fspec-tui/src/components/create_session_dialog.rs` (244 lines) — OK
  - `codelet/fspec-tui/src/app/dispatch_rpc060.rs` (124 lines) — OK
  - `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (Isolation arm) — OK
- Scenario coverage: 15/15

## Files Reviewed

- `spec/features/rpc060-isolated-session-dialog.feature`
- `codelet/fspec-tui/src/components/create_session_dialog.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc060.rs`
- `codelet/fspec-tui/src/app/dispatch.rs` (catch-all routing chain)
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (Isolation slash arm)
- `codelet/fspec-tui/src/components/mod.rs` (Action variants)
- `codelet/fspec-tui/src/components/dialog_theme.rs` (Accent type)
- `codelet/fspec-tui/src/lib.rs` (re-exports)
- `codelet/fspec-tui/tests/isolated_session_dialog_rpc060.rs`
- `codelet/fspec-tui/tests/source_shape_rpc060.rs`
- `codelet/fspec-tui/tests/common/mod.rs` (MockBackend RPC-060 plumbing)

---

## Implementation Quality

- ✅ Build: `cargo build -p codelet-fspec-tui` succeeds cleanly
- ✅ Tests: 22/22 pass (16 functional + 6 source-shape)
- ✅ No `todo!()`, `unimplemented!()`, `TODO`, `FIXME`, `HACK`, `XXX`
- ✅ All files under 300 lines (create_session_dialog.rs: 244, dispatch_rpc060.rs: 124, dispatch.rs: 299)
- ✅ All 6 example-map examples map to scenarios
- ✅ All 10 rules reflected in scenarios + implementation
- ✅ Architecture notes match implementation 1:1
- ✅ Out-of-scope items (SessionFooter, worktree merge, auto-isolated per-WU) confirmed untouched
- ✅ MockBackend exposes `seed_create_isolated_session_result` + `create_isolated_session_calls` as required by rule [9]
- ✅ Error path emits `[error] create isolated session: <e>` exactly matching rule [7]
- ✅ Idempotent dialog push (`if compositor.contains(...) { return; }`) in `handle_open_create_session_dialog`

---

## Fix Results

### RPC-060: Isolated session creation + AgentView /new isolated flow

- 🟡 W1 (accent assertion drift) → ✅ Fixed:
  - `codelet/fspec-tui/src/components/create_session_dialog.rs`: introduced
    module-level `const ACCENT: Accent = Accent::Cyan;` (single source of
    truth), exposed `pub fn accent(&self) -> Accent` accessor, and rewired
    `render()` to use `ACCENT` instead of the literal so the test assertion
    and the painted buffer cannot drift apart.
  - `codelet/fspec-tui/src/lib.rs`: re-exported `Accent` so external test code
    can assert against the canonical variant.
  - `codelet/fspec-tui/tests/isolated_session_dialog_rpc060.rs`: replaced
    `assert_eq!(dialog.priority(), Priority::Foreground)` /
    `assert_eq!(dialog.id(), CREATE_SESSION_DIALOG_ID)` under the
    "And the dialog accent is cyan" @step with the meaningful
    `assert_eq!(dialog.accent(), Accent::Cyan)`. The two prior assertions
    are retained as bonus invariants; they no longer masquerade as the
    accent assertion.
- 🟢 O1, O2, O3 → No change (acceptable observations only).

## Final Verification

- All tests pass: ✅ (16/16 + 6/6 RPC-060 tests; full `cargo test -p codelet-fspec-tui` clean)
- Build succeeds: ✅ (`cargo build -p codelet-fspec-tui` clean)
- Coverage complete: ✅ (15/15 scenarios)
- Feature file valid: ✅ (`fspec validate` clean)
- Tags valid: ✅
