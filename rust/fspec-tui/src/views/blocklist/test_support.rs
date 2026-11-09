//! Shared fixtures for the blocklist test modules (COV-075 split).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use codelet_rpc_types::BlocklistRuleInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::collections::HashSet;

pub(super) fn rule(id: &str, pattern: &str, action: &str, source: &str) -> BlocklistRuleInfo {
    BlocklistRuleInfo {
        id: id.to_string(),
        pattern: pattern.to_string(),
        action: action.to_string(),
        reason: String::new(),
        guidance: None,
        source: source.to_string(),
    }
}

/// Build `n` block rules with ids `rule000..`.
pub(super) fn rules(n: usize) -> Vec<BlocklistRuleInfo> {
    (0..n)
        .map(|i| {
            rule(
                &format!("rule{i:03}"),
                &format!("^cmd{i}"),
                "block",
                "system",
            )
        })
        .collect()
}

pub(super) fn press(view: &mut BlocklistView, code: KeyCode) {
    view.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// Render the view into a `TestBackend` buffer and return the joined
/// text of every row (used for buffer-text assertions).
pub(super) fn render_text(view: &mut BlocklistView, w: u16, h: u16) -> String {
    let disabled: HashSet<String> = HashSet::new();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("term");
    term.draw(|f| view.render(f.area(), f.buffer_mut(), &disabled))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// Count scrollbar glyphs (`■`/`│`) painted inside the LEFT list pane's
/// interior. BLOCK-009 adds a `render_vertical_divider` `│` column at the
/// pane boundary (~x = w/2 - 1); we stop BEFORE it so only the left
/// pane's own scrollbar gutter contributes — never the divider.
pub(super) fn scrollbar_glyphs(buf: &ratatui::buffer::Buffer, w: u16, h: u16) -> usize {
    let mut count = 0;
    // Left list pane only: stop one column short of the divider gutter.
    let x_start = 0u16;
    let x_end = (w / 2).saturating_sub(1);
    for y in 1..h.saturating_sub(1) {
        for x in x_start..x_end {
            let sym = buf[(x, y)].symbol();
            if sym == "■" || sym == "│" {
                count += 1;
            }
        }
    }
    count
}

// -------------------------------------------------------------------
// BLOCK-011 — mouse-wheel scroll support
// Feature: spec/features/blocklist-view-mouse-scroll.feature
//
// Level choice: the first four scenarios are exercised at the view
// level (in-module so we can read the private `selected_index` +
// `scroll_offset()` and drive `set_visible_rows`). The fifth
// (navigator-routing) scenario is exercised through the real
// `Navigator::handle_blocklist_event` via `crate::views::navigator`
// (mirroring the model_selector RPC-345 `switch_to_providers_flips_...`
// navigator test) — the strongest reliable assertion, since the
// navigator branch is the thin passthrough under test.
//
// WheelVelocity caveat: `WheelVelocity::step` accelerates on rapid
// consecutive events (1×–5× ramp). For a SINGLE event from a fresh
// view the step is exactly 1, so we assert exact index 1 deterministically.
// -------------------------------------------------------------------

/// Build a synthetic wheel/move MouseEvent. Column/row are irrelevant to
/// the wheel handler (it does no hit-testing), so we pin them at (1, 1).
pub(super) fn mouse_event(kind: crossterm::event::MouseEventKind) -> crossterm::event::MouseEvent {
    crossterm::event::MouseEvent {
        kind,
        column: 1,
        row: 1,
        modifiers: KeyModifiers::NONE,
    }
}
