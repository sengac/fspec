# PROV-134 — Left/Right arrow does not expand/collapse providers in `/provider` view

## Summary

In the Rust `/provider` (provider settings) list view, **Left and Right arrow keys are silently swallowed** and do nothing. The `/model` (model_selector) view supports **Left = collapse**, **Right = expand** on the focused provider header. The `/provider` view should behave the same way.

## Root Cause

`codelet/fspec-tui/src/views/provider_settings/list.rs` — function `handle_list_key` (lines ~27–151).

The `match key.code { ... }` block handles `Esc`, `Char('/')`, `PageDown/PageUp/Home/End`, `Tab`, `Up`, `Down`, `Enter`, `Char('d'/'D')`, and then a catch-all `_ => ProviderSettingsEvent::Consumed` (line ~149). There is **no `KeyCode::Left` arm and no `KeyCode::Right` arm** — both arrows hit the catch-all and are swallowed.

Currently expansion is only reachable via **Enter** on a Provider header row (`list_actions.rs::enter_on_nav_item`, `NavItemKind::Provider` arm → `view.toggle_expansion(&provider_id)`).

## Reference Behavior (already correct in `/model`)

`codelet/fspec-tui/src/views/model_selector/dispatch.rs` (lines ~97–104):

```rust
KeyCode::Left  => { self.toggle_expansion(false); ModelSelectorEvent::Consumed } // collapse
KeyCode::Right => { self.toggle_expansion(true);  ModelSelectorEvent::Consumed } // expand
```

Backed by `model_selector/navigation.rs::toggle_expansion(expand: bool)` (lines ~82–112) which does a **directional** `insert` (expand) / `remove` (collapse) plus cursor re-anchor.

TS reference for the same directional semantics: `src/tui/components/ModelSelectorScreen.tsx:192–201` — Right expands only when collapsed; Left collapses only when expanded.

## Infrastructure that already exists in provider_settings

- `expanded: HashSet<String>` state — `provider_settings/mod.rs`
- `toggle_expansion(provider_id: &str)` — `provider_settings/nav_tree_ops.rs` (lines ~109–116). NOTE: this is an **unconditional flip**, not directional.
- `focused_nav_item() -> Option<&NavItem>` — `nav_tree_ops.rs` (lines ~145–147)
- `NavItemKind::Provider { expanded: bool }` — `nav_item.rs:30`
- Render glyphs ▼/▶ — `list_nav_render.rs:215` / `row_render.rs`

## Fix

Add two arms to `handle_list_key` in `provider_settings/list.rs`, placed alongside the existing `Up`/`Down` arms (before the `_ =>` catch-all). Read `view.focused_nav_item()`; if it is `NavItemKind::Provider { expanded }`:

- **Right**: if NOT expanded → expand it.
- **Left**: if expanded → collapse it.

Because the existing `toggle_expansion` is an unconditional flip, guard on the current `expanded` state so Right only ever expands and Left only ever collapses (mirroring model_selector's set-on-Right / clear-on-Left semantics). Return `ProviderSettingsEvent::Consumed`.

Sketch:

```rust
KeyCode::Right => {
    if let Some(item) = view.focused_nav_item() {
        if let NavItemKind::Provider { expanded } = item.kind {
            if !expanded {
                let pid = item.provider_id.clone();
                view.toggle_expansion(&pid);
            }
        }
    }
    ProviderSettingsEvent::Consumed
}
KeyCode::Left => {
    if let Some(item) = view.focused_nav_item() {
        if let NavItemKind::Provider { expanded } = item.kind {
            if expanded {
                let pid = item.provider_id.clone();
                view.toggle_expansion(&pid);
            }
        }
    }
    ProviderSettingsEvent::Consumed
}
```

(Note: borrow the `provider_id` out before calling `&mut self` `toggle_expansion` to satisfy the borrow checker — copy fields out of the immutable `focused_nav_item()` borrow first.)

## Acceptance Criteria (Example-Mapping seeds)

- **Rule**: Right arrow on a collapsed provider header expands it.
- **Rule**: Right arrow on an already-expanded provider header is a no-op (no toggle).
- **Rule**: Left arrow on an expanded provider header collapses it.
- **Rule**: Left arrow on an already-collapsed provider header is a no-op.
- **Rule**: Arrow expand/collapse leaves the cursor on the same provider row.
- **Example**: Cursor on collapsed "OpenAI" header, press Right → OpenAI expands showing its child rows.
- **Example**: Cursor on expanded "OpenAI" header, press Left → OpenAI collapses hiding child rows.
- **Question (resolve during specifying)**: What should Left do when focus is on a CHILD row (not a header)? Model_selector re-anchors to the parent header and collapses it. Decide whether to match that or make it a no-op for the minimal fix.

## Files In Scope

- `codelet/fspec-tui/src/views/provider_settings/list.rs` (add two match arms)
- Tests: new `#[cfg(test)]` coverage for the two arms + no-op boundaries.

## Out of Scope

- Any change to `/model` behavior.
- Any change to Enter-to-toggle behavior.
