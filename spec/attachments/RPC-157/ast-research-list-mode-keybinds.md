# AST Research — RPC-157 List-mode Keybind Surface

Research conducted: 2026-06-01
Scope: `codelet/fspec-tui/src/views/provider_settings/**` (list-mode keyboard dispatcher)

---

## 1. Rust-only KeyCode arms that must be removed

`grep` pattern: `KeyCode::(PageUp|PageDown|Home|End|Char\('r'\)|Char\('R'\))`

```
codelet/fspec-tui/src/views/provider_settings/list.rs:59:        KeyCode::PageUp => {
codelet/fspec-tui/src/views/provider_settings/list.rs:64:        KeyCode::PageDown => {
codelet/fspec-tui/src/views/provider_settings/list.rs:69:        KeyCode::Home => {
codelet/fspec-tui/src/views/provider_settings/list.rs:74:        KeyCode::End => {
codelet/fspec-tui/src/views/provider_settings/detail.rs:56:        KeyCode::Char('r') | KeyCode::Char('R') => {
```

**Decision:** PageUp / PageDown / Home / End live in `list.rs` and ARE in scope for RPC-157 — delete their match arms (lines ~59-81). r/R lives in `detail.rs` (Detail::Summary mode); that arm is **out of scope** for RPC-157 — it will be removed when RPC-162 lands (which drops Detail::Summary entirely). RPC-157 only adds regression-guard tests asserting r/R remain no-ops in list mode (which they already are because list.rs has no r/R arm — they fall into the existing `_ => Consumed` catch-all).

---

## 2. wrap-around vs clamp arithmetic

`grep` pattern: `wrap_index|move_by`

```
codelet/fspec-tui/src/views/provider_settings/list.rs:52:            view.move_by(-1);
codelet/fspec-tui/src/views/provider_settings/list.rs:56:            view.move_by(1);
codelet/fspec-tui/src/views/provider_settings/list.rs:61:            view.move_by(-vr);
codelet/fspec-tui/src/views/provider_settings/list.rs:66:            view.move_by(vr);
codelet/fspec-tui/src/views/provider_settings/mod.rs:18:use crate::components::scroll_viewport::{ensure_visible, wrap_index};
codelet/fspec-tui/src/views/provider_settings/mod.rs:210:    pub(crate) fn move_by(&mut self, delta: i32) {
codelet/fspec-tui/src/views/provider_settings/mod.rs:215:        self.selected_index = wrap_index(self.selected_index, delta, total);
```

**Decision:** `move_by(delta)` is the single source of truth for arrow-driven cursor movement. It calls `wrap_index(...)` from `scroll_viewport`, which is the wrap-around helper used by every full-screen mode-view (ResumeSessionView etc.). To clamp instead of wrap, **introduce a new sibling helper `move_clamped(delta)`** on `ProviderSettingsView` and switch the two arrow arms (lines 51-58 of list.rs) to use it. Do NOT modify `move_by` because PageUp/PageDown call sites are being deleted anyway and we want to keep ResumeSessionView's wrap helper untouched (different view, different policy).

```rust
pub(crate) fn move_clamped(&mut self, delta: i32) {
    let total = self.visible_providers().len();
    if total == 0 {
        return;
    }
    let max_idx = (total - 1) as i32;
    let new_idx = (self.selected_index as i32 + delta).clamp(0, max_idx);
    self.selected_index = new_idx as usize;
    self.adjust_scroll();
}
```

After the change, `move_by` becomes unused inside provider_settings (PageUp/Down were the only other callers). The `move_by` method itself and the `wrap_index` import on mod.rs:18 must both be removed (or `move_by` kept as `#[allow(dead_code)]` if external callers exist — to verify in implementing phase via cargo check).

---

## 3. List-mode dispatcher exit shape

The remaining arms in `handle_list_key` after RPC-157 surgery:

```
KeyCode::Esc          → two-step cascade (clear filter, then Close)
KeyCode::Char('/')    → enter filter mode
KeyCode::Up           → move_clamped(-1)
KeyCode::Down         → move_clamped(1)
KeyCode::Enter        → transition to Detail (or expand provider per RPC-103)
KeyCode::Char('d') | KeyCode::Char('D') → open ConfirmDialog (if focused configured)
_                     → Consumed (silent no-op)
```

Tab is not yet bound in list.rs (RPC-160 will add it). Filter-mode key handling stays unchanged.

---

## 4. Existing test surface impact

`grep` for tests that assert wrap-around / PageUp / PageDown / Home / End behaviour:

`codelet/fspec-tui/tests/provider_settings_view_rpc054.rs`:
- `arrow_keys_wrap_selection_within_list` (asserts wrap-around) — DELETE in implementing phase
- `pagedown_pageup_home_end_mirror_resume_session_view` (asserts page/home/end jumps) — DELETE in implementing phase

Owning Gherkin scenarios in `spec/features/rpc054-provider-settings-view.feature`:
- Line ~45 "Arrow keys wrap selection within the providers list" — `fspec delete-scenario rpc054-provider-settings-view "Arrow keys wrap selection within the providers list"`
- Line ~67 "PageDown / PageUp / Home / End mirror ResumeSessionView jumps" — `fspec delete-scenario rpc054-provider-settings-view "PageDown / PageUp / Home / End mirror ResumeSessionView jumps"`

The other RPC-054 scenarios remain valid (`down_scrolls_window_past_visible_rows` still passes because clamped Down still calls `adjust_scroll`, just doesn't wrap).

---

## 5. Conclusion

Surgery is surgical and well-scoped. No new modules needed. Net change:
- Delete 4 KeyCode arms in `list.rs` (~26 LoC)
- Add `move_clamped` helper in `mod.rs` (~10 LoC)
- Switch 2 call sites (lines 52 & 56 of list.rs)
- Remove `move_by` + `wrap_index` import from mod.rs IF no other callers (verify with cargo check)
- Delete 2 obsolete tests + 2 obsolete Gherkin scenarios in RPC-054 feature file
- Add new test file `provider_settings_list_keybind_parity_rpc157.rs` with 9 scenarios

Estimated 2 story points still feels right.
